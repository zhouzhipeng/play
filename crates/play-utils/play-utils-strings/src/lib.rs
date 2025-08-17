use unicode_width::UnicodeWidthStr;

pub fn safe_substring(s: &str, start_char_index: usize, end_char_index: usize) -> String {
    let start_byte_index = s.char_indices()
        .nth(start_char_index)
        .map_or_else(|| s.len(), |(index, _)| index);
    let end_byte_index = s.char_indices()
        .nth(end_char_index)
        .map_or_else(|| s.len(), |(index, _)| index);

    s[start_byte_index..end_byte_index].to_string()
}


pub fn string_width_info(s: &str) -> (usize, bool) {
    let width = UnicodeWidthStr::width(s);
    let is_hundred = width == 100;
    (width, is_hundred)
}

pub fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut current_width = 0;
    let mut result = String::new();

    for c in s.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if current_width + char_width > max_width {
            break;
        }
        current_width += char_width;
        result.push(c);
    }

    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        println!("{}", safe_substring("你好sdfs", 0, 1));

        let test = "Hello你好World！";
        let (width, is_hundred) = string_width_info(test);
        println!("字符串宽度: {}", width);
        println!("是否为100宽度: {}", is_hundred);

        // 截断到指定宽度
        let truncated = truncate_to_width(test, 8);
        println!("截断到10宽度: {}", truncated);
    }
}
