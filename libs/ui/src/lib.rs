use wry::{
    application::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    },
    webview::WebViewBuilder,
};
use wry::application::dpi::LogicalSize;
use wry::application::window::Icon;


pub fn start_window(url: &str) -> wry::Result<()> {
    use wry::{
        application::{
            event::{Event, StartCause, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            window::WindowBuilder,
        },
        webview::WebViewBuilder,
    };

    use image::ImageFormat;

    //icon
    let bytes: Vec<u8> = include_bytes!("../icon.png").to_vec();
    let imagebuffer = image::load_from_memory_with_format(&bytes, ImageFormat::Png).unwrap().into_rgba8();
    let (icon_width, icon_height) = imagebuffer.dimensions();
    let icon_rgba = imagebuffer.into_raw();

    let icon = Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Play")
        .with_inner_size(LogicalSize::new(1000, 600))
        .with_window_icon(Some(icon.clone()))
        .build(&event_loop)?;
    let _webview = WebViewBuilder::new(window)?
        .with_url(url)?
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}
