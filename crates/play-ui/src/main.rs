use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use std::{env, fs};
use std::path::Path;
use std::net::TcpListener;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}};
use tokio::runtime::Runtime;
use webbrowser;

mod tray;
use tray::setup_tray;

static RUNNING: AtomicBool = AtomicBool::new(true);

fn find_available_port() -> u16 {
    // Try to bind to port 0, which will give us a random available port
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to port 0");
    let port = listener.local_addr().expect("Failed to get local address").port();
    drop(listener);
    port
}

fn ensure_config_and_database(data_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = Path::new(data_dir).join("config.toml");
    let db_path = Path::new(data_dir).join("play.db");
    
    if !config_path.exists() {
        println!("Config file not found, creating default config with random port");
        
        // Generate random available port
        let random_port = find_available_port();
        
        // Create SQLite database URL
        let db_url = format!("sqlite:///{}", db_path.display());
        
        // Create default config content
        let config_content = format!(
            r#"server_port = {}
log_level = "INFO"

[database]
url = "{}"

"#,
            random_port, db_url
        );
        
        // Write config file
        fs::write(&config_path, config_content)?;
        println!("Created config file with port {} and database {}", random_port, db_url);
    } else {
        // Check if existing config uses :memory: and update it
        let config_content = fs::read_to_string(&config_path)?;
        
        if config_content.contains(":memory:") || !config_content.contains(&data_dir) {
            println!("Updating config to use persistent SQLite database");
            
            // Parse as TOML to properly update
            let mut config_toml: toml::Value = toml::from_str(&config_content)?;
            
            // Update database URL
            let db_url = format!("sqlite:///{}", db_path.display());
            if let Some(database) = config_toml.get_mut("database") {
                if let Some(table) = database.as_table_mut() {
                    table.insert("url".to_string(), toml::Value::String(db_url.clone()));
                }
            }
            
            // Write back the updated config
            let updated_content = toml::to_string_pretty(&config_toml)?;
            fs::write(&config_path, updated_content)?;
            println!("Updated config file with persistent database URL: {}", db_url);
        }
    }
    
    // Ensure database file exists
    if !db_path.exists() {
        println!("Creating SQLite database at {:?}", db_path);
        fs::File::create(&db_path)?;
    }
    
    Ok(())
}

fn handle_existing_instance() -> bool {
    let current_pid = std::process::id();
    
    #[cfg(target_os = "macos")]
    {
        // On macOS, check if another instance is running
        if let Ok(output) = std::process::Command::new("pgrep")
            .arg("-f")
            .arg("play-ui")
            .output()
        {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid_str in pids.lines() {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if pid != current_pid {
                        println!("Another play-ui instance is already running (PID: {})", pid);
                        println!("Activating existing instance...");
                        
                        // Try to find the play-server port from the config
                        if let Ok(home) = env::var("HOME") {
                            let config_path = format!("{}/Library/Application Support/com.zhouzhipeng.play/config.toml", home);
                            if let Ok(config_content) = fs::read_to_string(&config_path) {
                                // Parse the config to find server_port
                                for line in config_content.lines() {
                                    if line.starts_with("server_port") {
                                        if let Some(port_str) = line.split('=').nth(1) {
                                            if let Ok(port) = port_str.trim().parse::<u16>() {
                                                let url = format!("http://127.0.0.1:{}", port);
                                                println!("Opening browser to activate existing instance: {}", url);
                                                let _ = webbrowser::open(&url);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        return true; // Signal that we should exit
                    }
                }
            }
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        // On other platforms, kill existing instances
        kill_existing_play_ui();
    }
    
    false // Continue with normal startup
}

fn kill_existing_play_ui() {
    let current_pid = std::process::id();
    let mut killed_any = false;
    
    #[cfg(unix)]
    {
        // Try to find and kill existing play-ui processes
        if let Ok(output) = std::process::Command::new("pgrep")
            .arg("-f")
            .arg("play-ui")
            .output()
        {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid_str in pids.lines() {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    if pid != current_pid {
                        println!("Terminating existing play-ui process: {}", pid);
                        killed_any = true;
                        
                        // First try SIGTERM (15) to allow graceful shutdown
                        let _ = std::process::Command::new("kill")
                            .arg("-15")
                            .arg(pid.to_string())
                            .output();
                        
                        // Wait a moment for graceful shutdown
                        thread::sleep(Duration::from_millis(1000));
                        
                        // Check if process still exists
                        if let Ok(check_output) = std::process::Command::new("kill")
                            .arg("-0")  // Signal 0 just checks if process exists
                            .arg(pid.to_string())
                            .output()
                        {
                            if check_output.status.success() {
                                // Process still exists, force kill it
                                println!("Force killing stubborn play-ui process: {}", pid);
                                let _ = std::process::Command::new("kill")
                                    .arg("-9")
                                    .arg(pid.to_string())
                                    .output();
                            }
                        }
                    }
                }
            }
        }
    }
    
    #[cfg(windows)]
    {
        // On Windows, use taskkill
        if let Ok(output) = std::process::Command::new("taskkill")
            .args(&["/F", "/IM", "play-ui.exe"])
            .output()
        {
            if output.status.success() {
                killed_any = true;
            }
        }
    }
    
    // Also try to kill any orphaned play-server processes
    #[cfg(unix)]
    {
        if let Ok(output) = std::process::Command::new("pgrep")
            .arg("-f")
            .arg("play-server")
            .output()
        {
            let pids = String::from_utf8_lossy(&output.stdout);
            for pid_str in pids.lines() {
                if let Ok(pid) = pid_str.trim().parse::<u32>() {
                    println!("Killing orphaned play-server process: {}", pid);
                    killed_any = true;
                    let _ = std::process::Command::new("kill")
                        .arg("-9")
                        .arg(pid.to_string())
                        .output();
                }
            }
        }
    }
    
    // Only wait if we actually killed any processes
    if killed_any {
        println!("Waiting for processes and system resources to be cleaned up...");
        thread::sleep(Duration::from_millis(2000));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Handle existing instances
    #[cfg(target_os = "macos")]
    {
        if handle_existing_instance() {
            // Another instance is running, exit gracefully
            println!("Exiting as another instance is already running.");
            return Ok(());
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    {
        kill_existing_play_ui();
    }

    
    // Initialize platform-specific settings first, before any async runtime
    #[cfg(target_os = "macos")]
    setup_macos_app();
    
    // Set up everything in a separate thread with tokio runtime
    let (config_tx, config_rx) = std::sync::mpsc::channel::<(String, String)>();
    
    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            if let Err(e) = async_main(config_tx).await {
                eprintln!("Error in async main: {}", e);
            }
        });
    });
    
    // Wait for server URL and data dir
    let (server_url, data_dir) = config_rx.recv()?;
    
    // Run tray on the main thread (required for macOS)
    run_tray(server_url, data_dir);
    
    Ok(())
}

async fn async_main(config_tx: std::sync::mpsc::Sender<(String, String)>) -> Result<(), Box<dyn std::error::Error>> {
    
    // Set up data directory
    #[cfg(not(feature = "debug"))]
    let data_dir = match directories::ProjectDirs::from("com", "zhouzhipeng", "play") {
        None => "/tmp/play".to_string(),
        Some(s) => s.data_dir().to_str().unwrap().to_string(),
    };
    
    #[cfg(feature = "debug")]
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("output_dir")
        .to_str()
        .unwrap()
        .to_string();
    
    env::set_var("DATA_DIR", &data_dir);
    println!("Using data dir: {:?}", data_dir);
    
    // Create directories if they don't exist
    fs::create_dir_all(&data_dir)?;
    fs::create_dir_all(Path::new(&data_dir).join("files"))?;
    
    // Ensure config and database exist with proper defaults
    ensure_config_and_database(&data_dir)?;
    
    // Get the server configuration
    let config = play_server::config::init_config(false).await?;
    let server_port = config.server_port;
    let server_url = format!("http://127.0.0.1:{}", server_port);
    
    println!("Starting Play Server on {}", server_url);
    

    // Clone values for the server thread
    let server_url_clone = server_url.clone();
    let data_dir_clone = data_dir.clone();

    // Start the server in a background thread
    let server_handle = tokio::spawn(async move {
        println!("Server starting at {}", server_url_clone);
        if let Err(e) = play_server::start_server_with_config(data_dir_clone, &config).await {
            eprintln!("Server error: {}", e);
        }
    });
    
    // Wait a moment for server to start
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Open homepage in browser
    println!("Opening homepage: {}", server_url);
    let _ = webbrowser::open(&server_url);
    
    // Send server URL and data_dir to main thread
    config_tx.send((server_url, data_dir))?;
    
    // Keep the server running
    tokio::select! {
        _ = server_handle => {
            println!("Server stopped");
            RUNNING.store(false, Ordering::Relaxed);
        }
        _ = tokio::signal::ctrl_c() => {
            println!("Received Ctrl+C, shutting down...");
            RUNNING.store(false, Ordering::Relaxed);
        }
    }
    
    Ok(())
}

fn run_tray(server_url: String, data_dir: String) {
    // IMPORTANT: Get the menu event receiver BEFORE creating the tray
    // This ensures we don't miss any early events
    let menu_channel = MenuEvent::receiver();
    
    // Create tray app - IMPORTANT: must keep a reference to prevent it from being dropped
    let _tray_icon = match setup_tray(&server_url) {
        Ok(app) => app,
        Err(e) => {
            eprintln!("Failed to create tray: {}", e);
            return;
        }
    };
    
    println!("System tray created successfully");
    println!("Server URL: {}", server_url);
    println!("Data DIR: {}", data_dir);
    
    // For macOS, we need to run the event loop
    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::{NSApp, NSApplication};
        use cocoa::base::nil;
        use objc::runtime::Object;
        use objc::*;
        
        // Set up a timer to check for menu events
        let data_dir_clone = data_dir.clone();
        thread::spawn(move || {
            println!("Menu event handler thread started");
            let mut last_event_time = std::time::Instant::now();
            
            while RUNNING.load(Ordering::Relaxed) {
                // Use try_recv instead of recv_timeout to avoid blocking
                match menu_channel.try_recv() {
                    Ok(event) => {
                        last_event_time = std::time::Instant::now();
                        println!("Menu event received: {}", event.id.0);
                    match event.id.0.as_str() {
                        "open_homepage" => {
                            println!("Opening homepage: {}", &server_url);
                            let _ = webbrowser::open(&server_url);
                        }
                        "open_data_dir" => {
                            println!("Opening data directory: {}", &data_dir_clone);
                                #[cfg(target_os = "macos")]
                                {
                                let _ = std::process::Command::new("open")
                                    .arg(&data_dir_clone)
                                    .spawn();
                            }
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("explorer")
                                    .arg(&data_dir_clone)
                                    .spawn();
                            }
                            #[cfg(target_os = "linux")]
                            {
                                let _ = std::process::Command::new("xdg-open")
                                    .arg(&data_dir_clone)
                                    .spawn();
                            }
                        }
                        "exit" => {
                            println!("Exit from menu");
                            RUNNING.store(false, Ordering::Relaxed);
                            unsafe {
                                let app = NSApp();
                                let () = msg_send![app, terminate: nil];
                            }
                        }
                        _ => {}
                    }
                    }
                    Err(_) => {
                        // No event, sleep a bit to avoid busy waiting
                        thread::sleep(Duration::from_millis(50));
                    }
                }
                
                // Log every 30 seconds to show the thread is still alive
                if last_event_time.elapsed() > Duration::from_secs(30) {
                    println!("Menu event handler still running, last event: {:?} ago", last_event_time.elapsed());
                    last_event_time = std::time::Instant::now();
                }
            }
            
            println!("Menu event handler thread exiting");
        });
        
        // Run the NSApplication event loop - this is required for the tray to work
        unsafe {
            let app = NSApp();
            app.run();
        }
    }
    
    // For other platforms
    #[cfg(not(target_os = "macos"))]
    {
        while RUNNING.load(Ordering::Relaxed) {
            if let Ok(event) = menu_channel.recv_timeout(Duration::from_millis(100)) {
                match event.id.0.as_str() {
                    "open_homepage" => {
                        println!("Opening homepage: {}", &server_url);
                        let _ = webbrowser::open(&server_url);
                    }
                    "open_data_dir" => {
                        println!("Opening data directory: {}", &data_dir);
                        #[cfg(target_os = "windows")]
                        {
                            let _ = std::process::Command::new("explorer")
                                .arg(&data_dir)
                                .spawn();
                        }
                        #[cfg(target_os = "linux")]
                        {
                            let _ = std::process::Command::new("xdg-open")
                                .arg(&data_dir)
                                .spawn();
                        }
                    }
                    "exit" => {
                        println!("Exit from menu");
                        RUNNING.store(false, Ordering::Relaxed);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn setup_macos_app() {
    use cocoa::base::nil;
    use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
    
    unsafe {
        let app = NSApp();
        if app == nil {
            panic!("Failed to initialize NSApplication");
        }
        
        // Initialize the app
        app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
    }
}

#[cfg(not(target_os = "macos"))]
fn setup_macos_app() {
    // No-op on other platforms
}