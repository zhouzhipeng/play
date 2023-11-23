use reqwest;
use plotters::prelude::*;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read, Write};

fn make_requests(url: &str, requests_count: usize) -> Vec<u128> {
    let client = reqwest::blocking::Client::new();
    let mut response_times = Vec::with_capacity(requests_count);

    for _ in 0..requests_count {
        let start_time = std::time::Instant::now();
        let response = client.get(url).send();

        match response {
            Ok(_) => {
                let duration = start_time.elapsed().as_micros();
                response_times.push(duration);
            }
            Err(e) => eprintln!("Error making request: {:?}", e),
        }
    }

    response_times
}

fn generate_graph(response_times: Vec<u128>) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new("response_times.png", (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let max_response_time = *response_times.iter().max().unwrap_or(&0);
    let mut chart = ChartBuilder::on(&root)
        .caption("Response Times", ("sans-serif", 30))
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0..response_times.len(), 0..max_response_time)?;

    chart.configure_mesh().draw()?;

    chart
        .draw_series(LineSeries::new(
            response_times.into_iter().enumerate().map(|(x, y)| (x, y)),
            &RED,
        ))?
        .label("Response Times")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart.configure_series_labels().draw()?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://www.example.com"; // Replace with your target URL
    let requests_count = 100; // Change this to the desired number of requests

    let response_times = make_requests(url, requests_count);

    generate_graph(response_times)?;

    // Read the generated PNG file and convert it to a base64 string
    let mut file = File::open("response_times.png")?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let base64_image = base64::encode(&buffer);

    // Create an HTML file with embedded image
    let html_content = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Response Times Graph</title>
        </head>
        <body>
            <h1>Response Times Graph</h1>
            <img src="data:image/png;base64,{}" />
        </body>
        </html>
    "#,
        base64_image
    );

    let mut html_file = File::create("response_times.html")?;
    html_file.write_all(html_content.as_bytes())?;

    Ok(())
}
