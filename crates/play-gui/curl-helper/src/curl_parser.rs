use eframe::egui;
use egui::text::LayoutJob;
use egui::{Color32, FontId, TextFormat};

#[derive(Debug, Clone)]
pub enum Token {
    Keyword(String),   // curl
    Flag(String),      // -X, -H, --data, etc.
    Url(String),       // URLs
    HeaderKey(String), // Header name part
    HeaderSep(String), // The ": " separator
    HeaderVal(String), // Header value part
    Method(String),    // GET, POST, PUT, DELETE, etc.
    DataValue(String), // Data payload
    Quote(String),     // Quote characters
    Whitespace(String),
    Backslash(String), // Line continuation
    Text(String),      // Other text
}

/// Check if a full string (e.g. inside quotes) looks like a URL.
fn looks_like_url(s: &str) -> bool {
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.contains("://")
        || s.starts_with("localhost")
        || (s.contains('/') && s.contains('?') && s.contains('='))
        || (s.contains(':') && s.contains('/') && !s.contains(' '))
}

/// Check if the beginning of a remaining string looks like a URL start (for unquoted detection).
fn looks_like_url_prefix(s: &str) -> bool {
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("localhost")
        || s.starts_with("127.0.0.1")
}

pub fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    // Track context for what comes after flags
    let mut expect_method = false;
    let mut expect_header = false;
    let mut expect_data = false;
    let mut expect_url = false;

    while i < len {
        // Whitespace
        if chars[i].is_whitespace() {
            let start = i;
            while i < len && chars[i].is_whitespace() {
                i += 1;
            }
            let ws: String = chars[start..i].iter().collect();
            tokens.push(Token::Whitespace(ws));
            continue;
        }

        // Line continuation backslash
        if chars[i] == '\\' && i + 1 < len && (chars[i + 1] == '\n' || chars[i + 1] == '\r') {
            let mut end = i + 1;
            if end < len && chars[end] == '\r' {
                end += 1;
            }
            if end < len && chars[end] == '\n' {
                end += 1;
            }
            let s: String = chars[i..end].iter().collect();
            tokens.push(Token::Backslash(s));
            i = end;
            continue;
        }

        // Handle what we expect after a flag
        if expect_method {
            expect_method = false;
            let start = i;
            // Could be quoted or unquoted
            if chars[i] == '\'' || chars[i] == '"' {
                let quote = chars[i];
                i += 1;
                let inner_start = i;
                while i < len && chars[i] != quote {
                    i += 1;
                }
                let method: String = chars[inner_start..i].iter().collect();
                if i < len {
                    i += 1; // skip closing quote
                }
                tokens.push(Token::Quote(quote.to_string()));
                tokens.push(Token::Method(method));
                tokens.push(Token::Quote(quote.to_string()));
            } else {
                while i < len && !chars[i].is_whitespace() {
                    i += 1;
                }
                let method: String = chars[start..i].iter().collect();
                tokens.push(Token::Method(method));
            }
            continue;
        }

        if expect_header {
            expect_header = false;
            if chars[i] == '\'' || chars[i] == '"' {
                let quote = chars[i];
                i += 1;
                tokens.push(Token::Quote(quote.to_string()));

                // Parse header key
                let key_start = i;
                while i < len && chars[i] != quote && chars[i] != ':' {
                    i += 1;
                }
                let key: String = chars[key_start..i].iter().collect();
                tokens.push(Token::HeaderKey(key));

                // Separator
                if i < len && chars[i] == ':' {
                    let sep_start = i;
                    i += 1;
                    if i < len && chars[i] == ' ' {
                        i += 1;
                    }
                    let sep: String = chars[sep_start..i].iter().collect();
                    tokens.push(Token::HeaderSep(sep));

                    // Value until closing quote
                    let val_start = i;
                    while i < len && chars[i] != quote {
                        i += 1;
                    }
                    let val: String = chars[val_start..i].iter().collect();
                    tokens.push(Token::HeaderVal(val));
                }

                if i < len && chars[i] == quote {
                    tokens.push(Token::Quote(quote.to_string()));
                    i += 1;
                }
            } else {
                // Unquoted header - read until whitespace
                let start = i;
                while i < len && !chars[i].is_whitespace() {
                    i += 1;
                }
                let h: String = chars[start..i].iter().collect();
                if let Some(colon_pos) = h.find(':') {
                    tokens.push(Token::HeaderKey(h[..colon_pos].to_string()));
                    let sep_end = if h.get(colon_pos + 1..colon_pos + 2) == Some(" ") {
                        colon_pos + 2
                    } else {
                        colon_pos + 1
                    };
                    tokens.push(Token::HeaderSep(h[colon_pos..sep_end].to_string()));
                    tokens.push(Token::HeaderVal(h[sep_end..].to_string()));
                } else {
                    tokens.push(Token::Text(h));
                }
            }
            continue;
        }

        if expect_data {
            expect_data = false;
            if chars[i] == '\'' || chars[i] == '"' {
                let quote = chars[i];
                i += 1;
                let start = i;
                let mut depth: u32 = 0;
                while i < len && !(chars[i] == quote && depth == 0) {
                    if chars[i] == '{' || chars[i] == '[' {
                        depth += 1;
                    }
                    if chars[i] == '}' || chars[i] == ']' {
                        depth = depth.saturating_sub(1);
                    }
                    if chars[i] == '\\' && i + 1 < len {
                        i += 1;
                    }
                    i += 1;
                }
                let data: String = chars[start..i].iter().collect();
                tokens.push(Token::Quote(quote.to_string()));
                tokens.push(Token::DataValue(data));
                if i < len {
                    tokens.push(Token::Quote(quote.to_string()));
                    i += 1;
                }
            } else {
                let start = i;
                while i < len && !chars[i].is_whitespace() {
                    i += 1;
                }
                let data: String = chars[start..i].iter().collect();
                tokens.push(Token::DataValue(data));
            }
            continue;
        }

        if expect_url {
            expect_url = false;
            if chars[i] == '\'' || chars[i] == '"' {
                let quote = chars[i];
                i += 1;
                let start = i;
                while i < len && chars[i] != quote {
                    i += 1;
                }
                let url: String = chars[start..i].iter().collect();
                tokens.push(Token::Quote(quote.to_string()));
                tokens.push(Token::Url(url));
                if i < len {
                    tokens.push(Token::Quote(quote.to_string()));
                    i += 1;
                }
            } else {
                let start = i;
                while i < len && !chars[i].is_whitespace() {
                    i += 1;
                }
                let url: String = chars[start..i].iter().collect();
                tokens.push(Token::Url(url));
            }
            continue;
        }

        // Flags
        if chars[i] == '-' {
            let start = i;
            i += 1;
            if i < len && chars[i] == '-' {
                // Long flag
                i += 1;
                while i < len && (chars[i].is_alphanumeric() || chars[i] == '-') {
                    i += 1;
                }
            } else {
                // Short flag
                while i < len && chars[i].is_alphanumeric() {
                    i += 1;
                }
            }
            let flag: String = chars[start..i].iter().collect();

            match flag.as_str() {
                "-X" | "--request" => {
                    expect_method = true;
                    tokens.push(Token::Flag(flag));
                }
                "-H" | "--header" => {
                    expect_header = true;
                    tokens.push(Token::Flag(flag));
                }
                "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-urlencode" => {
                    expect_data = true;
                    tokens.push(Token::Flag(flag));
                }
                "--url" => {
                    expect_url = true;
                    tokens.push(Token::Flag(flag));
                }
                _ => {
                    tokens.push(Token::Flag(flag));
                }
            }
            continue;
        }

        // Check for "curl" keyword
        if i + 4 <= len {
            let word: String = chars[i..i + 4].iter().collect();
            if word == "curl" && (i + 4 >= len || !chars[i + 4].is_alphanumeric()) {
                tokens.push(Token::Keyword("curl".to_string()));
                i += 4;
                continue;
            }
        }

        // Quoted strings that look like URLs
        if chars[i] == '\'' || chars[i] == '"' {
            let quote = chars[i];
            i += 1;
            let start = i;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < len {
                    i += 1;
                }
                i += 1;
            }
            let content: String = chars[start..i].iter().collect();
            if i < len {
                i += 1;
            }
            if looks_like_url(&content) {
                tokens.push(Token::Quote(quote.to_string()));
                tokens.push(Token::Url(content));
                tokens.push(Token::Quote(quote.to_string()));
            } else {
                tokens.push(Token::Quote(quote.to_string()));
                tokens.push(Token::Text(content));
                tokens.push(Token::Quote(quote.to_string()));
            }
            continue;
        }

        // URLs (unquoted)
        let remaining: String = chars[i..].iter().collect();
        if looks_like_url_prefix(&remaining) {
            let start = i;
            while i < len && !chars[i].is_whitespace() {
                i += 1;
            }
            let url: String = chars[start..i].iter().collect();
            tokens.push(Token::Url(url));
            continue;
        }

        // Other text
        let start = i;
        while i < len && !chars[i].is_whitespace() && chars[i] != '-' && chars[i] != '\'' && chars[i] != '"' && chars[i] != '\\' {
            i += 1;
        }
        if i == start {
            // Single character we didn't match
            tokens.push(Token::Text(chars[i].to_string()));
            i += 1;
        } else {
            let text: String = chars[start..i].iter().collect();
            tokens.push(Token::Text(text));
        }
    }

    tokens
}

pub fn highlight(text: &str) -> LayoutJob {
    let mut job = LayoutJob::default();
    let tokens = tokenize(text);

    let mono = FontId::monospace(14.0);

    let keyword_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(86, 156, 214), // blue
        ..Default::default()
    };
    let flag_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(220, 220, 170), // yellow
        ..Default::default()
    };
    let url_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(78, 201, 176), // teal
        ..Default::default()
    };
    let method_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(229, 113, 84), // red-orange
        ..Default::default()
    };
    let header_key_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(156, 220, 254), // light blue
        ..Default::default()
    };
    let header_sep_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(180, 180, 180), // gray
        ..Default::default()
    };
    let header_val_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(206, 145, 120), // orange
        ..Default::default()
    };
    let data_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(181, 206, 168), // green
        ..Default::default()
    };
    let quote_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(120, 120, 120), // dim gray
        ..Default::default()
    };
    let backslash_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(100, 100, 100),
        ..Default::default()
    };
    let param_key_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(156, 220, 254), // light blue (same as header key)
        ..Default::default()
    };
    let param_val_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(255, 180, 100), // warm orange
        ..Default::default()
    };
    let param_sep_fmt = TextFormat {
        font_id: mono.clone(),
        color: Color32::from_rgb(150, 150, 150), // gray
        ..Default::default()
    };
    let default_fmt = TextFormat {
        font_id: mono,
        color: Color32::from_rgb(212, 212, 212), // light gray
        ..Default::default()
    };

    for token in &tokens {
        match token {
            Token::Keyword(s) => job.append(s, 0.0, keyword_fmt.clone()),
            Token::Flag(s) => job.append(s, 0.0, flag_fmt.clone()),
            Token::Url(s) => {
                // Highlight URL query params: key=value&key=value
                if let Some(q) = s.find('?') {
                    job.append(&s[..q], 0.0, url_fmt.clone());
                    job.append("?", 0.0, param_sep_fmt.clone());
                    let query = &s[q + 1..];
                    for (pi, pair) in query.split('&').enumerate() {
                        if pi > 0 {
                            job.append("&", 0.0, param_sep_fmt.clone());
                        }
                        if let Some(eq) = pair.find('=') {
                            job.append(&pair[..eq], 0.0, param_key_fmt.clone());
                            job.append("=", 0.0, param_sep_fmt.clone());
                            job.append(&pair[eq + 1..], 0.0, param_val_fmt.clone());
                        } else {
                            job.append(pair, 0.0, param_key_fmt.clone());
                        }
                    }
                } else {
                    job.append(s, 0.0, url_fmt.clone());
                }
            }
            Token::Method(s) => job.append(s, 0.0, method_fmt.clone()),
            Token::HeaderKey(s) => job.append(s, 0.0, header_key_fmt.clone()),
            Token::HeaderSep(s) => job.append(s, 0.0, header_sep_fmt.clone()),
            Token::HeaderVal(s) => job.append(s, 0.0, header_val_fmt.clone()),
            Token::DataValue(s) => job.append(s, 0.0, data_fmt.clone()),
            Token::Quote(s) => job.append(s, 0.0, quote_fmt.clone()),
            Token::Backslash(s) => job.append(s, 0.0, backslash_fmt.clone()),
            Token::Whitespace(s) => job.append(s, 0.0, default_fmt.clone()),
            Token::Text(s) => job.append(s, 0.0, default_fmt.clone()),
        }
    }

    job
}

/// Extract key-value parameters from a curl command for the parameter panel.
/// Returns (category, key, value) triples.
pub fn extract_params(input: &str) -> Vec<(String, String, String)> {
    let mut params = Vec::new();
    let tokens = tokenize(input);

    // Walk tokens to find URLs and headers
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Url(url) => {
                if let Some(q) = url.find('?') {
                    let query = &url[q + 1..];
                    for pair in query.split('&') {
                        if let Some(eq) = pair.find('=') {
                            params.push((
                                "Query".to_string(),
                                pair[..eq].to_string(),
                                pair[eq + 1..].to_string(),
                            ));
                        }
                    }
                }
            }
            Token::HeaderKey(key) => {
                // Look ahead for the value
                let mut val = String::new();
                for j in (i + 1)..tokens.len() {
                    if let Token::HeaderVal(v) = &tokens[j] {
                        val = v.clone();
                        break;
                    }
                    if matches!(&tokens[j], Token::Whitespace(_) | Token::Flag(_) | Token::Keyword(_)) {
                        break;
                    }
                }
                if !val.is_empty() {
                    params.push(("Header".to_string(), key.clone(), val));
                }
            }
            Token::Method(m) => {
                params.push(("Method".to_string(), "method".to_string(), m.clone()));
            }
            Token::DataValue(d) => {
                let trimmed = d.trim();
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    // Unescape \" → " for JSON inside double-quoted shell strings
                    let unescaped = trimmed.replace("\\\"", "\"");
                    // Parse JSON body and flatten to key-value pairs
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&unescaped) {
                        flatten_json("", &json, &mut params);
                    }
                } else if d.contains('=') {
                    // Form-urlencoded: key=val&key=val
                    for pair in d.split('&') {
                        if let Some(eq) = pair.find('=') {
                            params.push((
                                "Data".to_string(),
                                pair[..eq].to_string(),
                                pair[eq + 1..].to_string(),
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    params
}

/// Flatten a JSON value into (category, key_path, value) triples.
fn flatten_json(prefix: &str, val: &serde_json::Value, params: &mut Vec<(String, String, String)>) {
    match val {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() { k.clone() } else { format!("{}.{}", prefix, k) };
                match v {
                    serde_json::Value::Object(_) | serde_json::Value::Array(_) => {
                        flatten_json(&key, v, params);
                    }
                    _ => {
                        let val_str = match v {
                            serde_json::Value::String(s) => s.clone(),
                            _ => v.to_string(),
                        };
                        params.push(("Body".to_string(), key, val_str));
                    }
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let key = format!("{}[{}]", prefix, i);
                flatten_json(&key, v, params);
            }
        }
        _ => {
            let val_str = match val {
                serde_json::Value::String(s) => s.clone(),
                _ => val.to_string(),
            };
            if !prefix.is_empty() {
                params.push(("Body".to_string(), prefix.to_string(), val_str));
            }
        }
    }
}

/// Replace a specific parameter value in the command.
/// Handles both `key=old_value` (query/form) and JSON `"key": old_value` patterns.
pub fn replace_param_value(command: &str, key: &str, old_value: &str, new_value: &str) -> String {
    // Extract the leaf key (last segment after '.' or '[')
    let leaf = key.rsplit('.').next().unwrap_or(key);
    let leaf = leaf.split('[').next().unwrap_or(leaf);

    // Try JSON patterns: "leaf": "old" or "leaf": old (number)
    // String value
    let find_str = format!("\"{}\": \"{}\"", leaf, old_value);
    let replace_str = format!("\"{}\": \"{}\"", leaf, new_value);
    if command.contains(&find_str) {
        return command.replacen(&find_str, &replace_str, 1);
    }
    // Without space after colon
    let find_str2 = format!("\"{}\":\"{}\"", leaf, old_value);
    let replace_str2 = format!("\"{}\":\"{}\"", leaf, new_value);
    if command.contains(&find_str2) {
        return command.replacen(&find_str2, &replace_str2, 1);
    }
    // Numeric value
    let find_num = format!("\"{}\": {}", leaf, old_value);
    let replace_num = format!("\"{}\": {}", leaf, new_value);
    if command.contains(&find_num) {
        return command.replacen(&find_num, &replace_num, 1);
    }
    let find_num2 = format!("\"{}\":{}", leaf, old_value);
    let replace_num2 = format!("\"{}\":{}", leaf, new_value);
    if command.contains(&find_num2) {
        return command.replacen(&find_num2, &replace_num2, 1);
    }

    // Fallback: query/form style key=value
    let find = format!("{}={}", key, old_value);
    let replace = format!("{}={}", key, new_value);
    command.replacen(&find, &replace, 1)
}

/// Replace a value in curl commands - finds all occurrences of old_value and replaces with new_value
pub fn replace_in_command(command: &str, find: &str, replace: &str) -> String {
    command.replace(find, replace)
}
