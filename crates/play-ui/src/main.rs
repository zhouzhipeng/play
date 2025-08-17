use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use std::{env, fs};
use std::path::Path;
use std::net::TcpListener;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}};
use tokio::runtime::Runtime;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize platform-specific settings first, before any async runtime
    #[cfg(target_os = "macos")]
    setup_macos_app();
    
    // Set up everything in a separate thread with tokio runtime
    let (server_url_tx, server_url_rx) = std::sync::mpsc::channel();
    
    thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            if let Err(e) = async_main(server_url_tx).await {
                eprintln!("Error in async main: {}", e);
            }
        });
    });
    
    // Wait for server URL
    let server_url = server_url_rx.recv()?;
    
    // Run tray on the main thread (required for macOS)
    run_tray(server_url);
    
    Ok(())
}

async fn async_main(server_url_tx: std::sync::mpsc::Sender<String>) -> Result<(), Box<dyn std::error::Error>> {
    
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

    // Start the server in a background thread
    let server_handle = tokio::spawn(async move {
        println!("Server starting at {}", server_url_clone);
        if let Err(e) = play_server::start_server_with_config(data_dir, &config).await {
            eprintln!("Server error: {}", e);
        }
    });
    
    // Wait a moment for server to start
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Open homepage in browser
    println!("Opening homepage: {}", server_url);
    let _ = webbrowser::open(&server_url);
    
    // Send server URL to main thread
    server_url_tx.send(server_url)?;
    
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

fn run_tray(server_url: String) {
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
    
    // Handle menu events
    let menu_channel = MenuEvent::receiver();
    
    // For macOS, we need to run the event loop
    #[cfg(target_os = "macos")]
    {
        use cocoa::appkit::{NSApp, NSApplication};
        use cocoa::base::nil;
        use objc::runtime::Object;
        use objc::*;
        
        // Set up a timer to check for menu events
        thread::spawn(move || {
            while RUNNING.load(Ordering::Relaxed) {
                if let Ok(event) = menu_channel.recv_timeout(Duration::from_millis(100)) {
                    match event.id.0.as_str() {
                        "open_homepage" => {
                            println!("Opening homepage: {}", &server_url);
                            let _ = webbrowser::open(&server_url);
                        }
                        "open_data_dir" => {
                            if let Ok(data_dir) = env::var("DATA_DIR") {
                                println!("Opening data directory: {}", data_dir);
                                #[cfg(target_os = "macos")]
                                {
                                    let _ = std::process::Command::new("open")
                                        .arg(&data_dir)
                                        .spawn();
                                }
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
            }
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
                        if let Ok(data_dir) = env::var("DATA_DIR") {
                            println!("Opening data directory: {}", data_dir);
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