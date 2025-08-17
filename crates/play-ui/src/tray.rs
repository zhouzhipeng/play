use tray_icon::{Icon, TrayIcon, TrayIconBuilder, menu::{Menu, MenuItem}};
use image;

pub fn setup_tray(server_url: &str) -> Result<TrayIcon, Box<dyn std::error::Error>> {
    println!("Setting up tray icon...");
    let menu = create_menu();
    println!("Menu created");
    let icon = create_icon();
    println!("Icon created");
    
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(&format!("Play Server - {}", server_url))
        .with_icon(icon)
        .build()?;
    
    println!("Tray icon built successfully");
    
    // On macOS, we need to show the tray icon
    #[cfg(target_os = "macos")]
    {
        tray_icon.set_visible(true)?;
        println!("Tray icon set to visible");
    }
    
    Ok(tray_icon)
}

fn create_menu() -> Menu {
    let menu = Menu::new();
    
    let open_homepage = MenuItem::with_id("open_homepage", "打开首页", true, None);
    let open_data_dir = MenuItem::with_id("open_data_dir", "打开数据目录", true, None);
    let exit = MenuItem::with_id("exit", "退出", true, None);
    
    menu.append(&open_homepage).unwrap();
    menu.append(&open_data_dir).unwrap();
    menu.append(&exit).unwrap();
    
    menu
}

fn create_icon() -> Icon {
    // Try to load icon from file
    let icon_paths = vec![
        "icon.png",
        "crates/play-ui/icon.png",
        "/Users/ronnie/RustroverProjects/play/crates/play-ui/icon.png",
    ];
    
    for path in icon_paths {
        if let Ok(icon_data) = std::fs::read(path) {
            if let Ok(img) = image::load_from_memory(&icon_data) {
                let rgba = img.to_rgba8();
                let (width, height) = (rgba.width(), rgba.height());
                
                // Resize to 32x32 if needed
                let resized = if width != 32 || height != 32 {
                    image::imageops::resize(&rgba, 32, 32, image::imageops::FilterType::Lanczos3)
                } else {
                    rgba
                };
                
                let icon_data = resized.into_raw();
                if let Ok(icon) = Icon::from_rgba(icon_data, 32, 32) {
                    return icon;
                }
            }
        }
    }
    
    // Create default icon if file not found
    create_default_icon()
}

fn create_default_icon() -> Icon {
    let size = 32;
    let mut icon_data = Vec::with_capacity((size * size * 4) as usize);
    
    for y in 0..size {
        for x in 0..size {
            // Create a gradient icon
            let r = ((x as f32 / size as f32) * 50.0 + 50.0) as u8;
            let g = ((y as f32 / size as f32) * 100.0 + 100.0) as u8;
            let b = 200u8;
            
            icon_data.push(r);
            icon_data.push(g);
            icon_data.push(b);
            icon_data.push(255);
        }
    }
    
    Icon::from_rgba(icon_data, size, size).expect("Failed to create icon")
}