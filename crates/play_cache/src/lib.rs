use anyhow::Result;
use headless_chrome::{Browser, LaunchOptions, LaunchOptionsBuilder};
use std::time::Duration;
use tokio::task::JoinHandle;
use log::info;

pub async fn render_html_in_browser(url: &str) -> Result<String> {
    let url = url.to_string();
    let _:JoinHandle<Result<String>>  = tokio::spawn(async move {
        let options = LaunchOptions {
            headless: true,
            port: Some(8989),
            sandbox: false,
            // 设置为使用一个线程
            process_envs: Some(vec![("CHROMIUM_FLAGS".to_string(), "--single-process".to_string())].into_iter().collect()),
            ..Default::default()
        };

        // Launch the browser
        let browser = Browser::new(
            options,
        )?;

        // Create a new tab
        let tab = browser.new_tab()?;

        // Navigate to the page
        tab.navigate_to(&url)?;

        // Wait for the page to load
        tab.wait_until_navigated()?;

        // Wait for network to be idle (adjust timeout as needed)
        tab.wait_for_element_with_custom_timeout("body", Duration::from_secs(10))?;

        // Additional wait to ensure WASM execution (adjust as needed)
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Get the rendered HTML
        let html_content = tab.get_content()?;
        info!("html content: {}", html_content);

        Ok(html_content)
    });

    Ok("ok".to_string())
}


#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    use super::*;

    #[tokio::test]
    async fn test_parse() -> Result<()> {

        println!("{}", render_html_in_browser("http://example.com/index.html").await?);

        Ok(())
    }
}