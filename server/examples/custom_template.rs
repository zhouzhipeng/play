use std::collections::HashMap;
use regex::Regex;

struct Template {
    code_lines: Vec<String>,
    code_blocks: Vec<String>,
    inline_placeholders: Vec<String>,
}

impl Template {
    fn new() -> Self {
        Template {
            code_lines: Vec::new(),
            code_blocks: Vec::new(),
            inline_placeholders: Vec::new(),
        }
    }

    fn parse(&mut self, template_str: &str) {
        let re_code_lines = Regex::new(r"^\s*%(.*?)$").unwrap();
        let re_code_blocks = Regex::new(r"<%(.*?)%>").unwrap();
        let re_inline_placeholders = Regex::new(r"\{\{(.+?)\}\}").unwrap();

        self.code_lines = template_str
            .lines()
            .filter_map(|line| {
                re_code_lines
                    .captures(line)
                    .map(|cap| cap[1].trim().to_string())
            })
            .collect();

        self.code_blocks = re_code_blocks
            .captures_iter(template_str)
            .map(|cap| cap[1].trim().to_string())
            .collect();

        self.inline_placeholders = re_inline_placeholders
            .captures_iter(template_str)
            .map(|cap| cap[1].trim().to_string())
            .collect();
    }

    fn render(&self, context: &HashMap<&str, &str>) -> String {
        let mut result = String::new();

        for line in &self.code_lines {
            result.push_str(line);
            result.push('\n');
        }

        for block in &self.code_blocks {
            // Execute code blocks
            let code = block.replace("{{", "").replace("}}", "");
            if !code.contains("{{") && !code.contains("}}") {
                result.push_str(&code);
                result.push('\n');
            } else {
                panic!("Error: Code block cannot contain inline placeholders");
            }
        }

        for placeholder in &self.inline_placeholders {
            if let Some(value) = context.get(placeholder.as_str()) {
                result = result.replace(&format!("{{{{ {} }}}}", placeholder), value);
            }
        }

        result
    }
}

fn main() {
    let mut template = Template::new();
    let html_template = r#"
        <html>
            <head>
                <title>{{ title }}</title>
            </head>
            <body>
                % let greeting = "Welcome to the page";
                % let emphasis = "!";
                <h1>{{ greeting }}{{ emphasis }}</h1>
                <div>
                    <% for item in items.iter() {
                            println!("{}", item); // Example code
                    } %>
                </div>
            </body>
        </html>
    "#;

    template.parse(html_template);

    let mut context = HashMap::new();
    context.insert("title", "Sample Title");
    context.insert("greeting", "Welcome message");
    context.insert("emphasis", "!!");
    context.insert("items", "Item 1\nItem 2\nItem 3");

    let rendered = template.render(&context);
    println!("{}", rendered);
}
