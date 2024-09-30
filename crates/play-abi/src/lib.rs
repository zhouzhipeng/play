pub mod http_abi;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
pub fn c_char_to_string(c_str: *const c_char) -> String {
    unsafe {
        CStr::from_ptr(c_str) // 从指针创建 C 风格字符串
            .to_string_lossy() // 转换为 Rust String，处理无效 UTF-8 的情况
            .into_owned() // 获取 String 的所有权
    }
}


pub  fn string_to_c_char(rust_string: &str) -> *const c_char {
    CString::new(rust_string) // 创建 CString
        .expect("Failed to create CString") // 确保输入字符串中没有 null 字符
        .into_raw() // 转换为原始指针
}
pub  fn string_to_c_char_mut(rust_string: &str) -> *mut c_char {
    CString::new(rust_string) // 创建 CString
        .expect("Failed to create CString") // 确保输入字符串中没有 null 字符
        .into_raw() // 转换为原始指针
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn test_convert(){
        // Rust String to *const c_char
        let rust_str = "Hello, World!你好啊";
        let c_str = string_to_c_char(rust_str);
        println!("C String: {:?}", c_str);

        // *const c_char back to Rust String
        let back_to_rust = c_char_to_string(c_str);
        println!("Back to Rust String: {}", back_to_rust);
    }

}