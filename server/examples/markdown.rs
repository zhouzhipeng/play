fn main() {
    // Create parser with example Markdown text.
    let markdown_input = include_str!("test.md");
    let parser = pulldown_cmark::Parser::new(markdown_input);

// Write to a new String buffer.
    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, parser);
    // assert_eq!(&html_output, "<p>hello world</p>\n");
    println!("{}", html_output);
}