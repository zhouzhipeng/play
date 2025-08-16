use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, menu::{Menu, MenuItem, MenuEvent}};

static RUNNING: AtomicBool = AtomicBool::new(true);

pub struct TrayApp {
    _tray_icon: TrayIcon,
    homepage_url: String,
}

impl TrayApp {
    pub fn new(homepage_url: String) -> Result<Self, Box<dyn std::error::Error>> {
        let menu = Self::create_menu();
        let icon = Self::create_icon();
        
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Play Server")
            .with_icon(icon)
            .build()?;
        
        Ok(Self {
            _tray_icon: tray_icon,
            homepage_url,
        })
    }
    
    fn create_menu() -> Menu {
        let menu = Menu::new();
        
        let open_homepage = MenuItem::with_id("open_homepage", "打开首页", true, None);
        let exit = MenuItem::with_id("exit", "退出", true, None);
        
        menu.append(&open_homepage).unwrap();
        menu.append(&exit).unwrap();
        
        menu
    }
    
    fn create_icon() -> Icon {
        // Try to load icon from file
        if let Ok(icon_data) = std::fs::read("/Users/ronnie/RustroverProjects/play/crates/play-ui/icon.png") {
            if let Ok(img) = image::load_from_memory(&icon_data) {
                let rgba = img.to_rgba8();
                let (width, height) = (rgba.width(), rgba.height());
                let icon_data = rgba.into_raw();
                if let Ok(icon) = Icon::from_rgba(icon_data, width, height) {
                    return icon;
                }
            }
        }
        
        // Create default icon if file not found
        Self::create_default_icon()
    }
    
    fn create_default_icon() -> Icon {
        let size = 32;
        let mut icon_data = Vec::with_capacity((size * size * 4) as usize);
        
        for y in 0..size {
            for x in 0..size {
                // Create a gradient icon
                let r = ((x as f32 / size as f32) * 100.0) as u8;
                let g = ((y as f32 / size as f32) * 150.0) as u8;
                let b = 200u8;
                
                icon_data.push(r);
                icon_data.push(g);
                icon_data.push(b);
                icon_data.push(255);
            }
        }
        
        Icon::from_rgba(icon_data, size, size).expect("Failed to create icon")
    }
    
    pub fn run(self) {
        let homepage_url = self.homepage_url.clone();
        
        // Handle menu events
        let menu_channel = MenuEvent::receiver();
        
        println!("Play Server UI 已启动，系统托盘图标已创建");
        println!("服务器地址: {}", &homepage_url);
        println!("点击托盘图标可以打开菜单");
        
        while RUNNING.load(Ordering::Relaxed) {
            if let Ok(event) = menu_channel.recv_timeout(Duration::from_millis(100)) {
                match event.id.0.as_str() {
                    "open_homepage" => {
                        println!("打开首页: {}", &homepage_url);
                        let _ = webbrowser::open(&homepage_url);
                    }
                    "exit" => {
                        println!("退出程序...");
                        RUNNING.store(false, Ordering::Relaxed);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn start_window(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize platform-specific settings
    init();
    
    // Open homepage on startup
    println!("打开首页: {}", url);
    let _ = webbrowser::open(url);
    
    // Wait a moment for browser to open
    thread::sleep(Duration::from_millis(500));
    
    // Create and run tray app
    let app = TrayApp::new(url.to_string())?;
    
    // Run the tray application
    app.run();
    
    Ok(())
}

// Platform-specific initialization
pub fn init() {
    #[cfg(target_os = "macos")]
    setup_macos_app();
}

#[cfg(target_os = "macos")]
fn setup_macos_app() {
    use cocoa::base::nil;
    use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
    
    unsafe {
        let app = NSApp();
        // Set as accessory app (no dock icon, only menu bar)
        app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
    }
}

#[cfg(not(target_os = "macos"))]
fn setup_macos_app() {
    // No-op on other platforms
}