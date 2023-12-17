use std::io::{Error, ErrorKind, Write};

/// Internal utility for writing data into a string
/// Internal utility for writing data into a string
pub struct StringWriter {
    string: String,
}

impl StringWriter {
    /// Create a new `StringWriter`
    pub fn new() -> StringWriter {
        StringWriter {
            string: String::new(),
        }
    }

    /// Return a reference to the internally written `String`
    pub fn as_string(&self) -> &str {
        &self.string
    }
}
use std::{fs, str};
impl Write for StringWriter {
    fn write(&mut self, data: &[u8]) -> Result<usize, Error> {
        let string = match str::from_utf8(data) {
            Ok(s) => s,
            Err(e) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    format!("Cannot decode utf8 string : {}", e),
                ))
            }
        };
        self.string.push_str(string);
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<(), Error> {
        // Nothing to do here
        Ok(())
    }
}



fn main() {
    let left_data = "abc";
    let left_name = "1.txt";

    let right_data = "abcd";
    let right_name = "2.txt";

    let table = prettydiff::diff_lines(&left_data, &right_data)
        .names(&left_name, &right_name)
        .set_show_lines(true)
        .set_diff_only(false)
        .set_align_new_lines(true)
        .prettytable();

    //
    let mut buf = StringWriter::new();

    table.print_html(&mut buf).expect("failed");
    println!("{}", buf.as_string());

    fs::write("diff.html", buf.as_string()).expect("ss");
}