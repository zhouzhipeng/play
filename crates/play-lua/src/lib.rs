use std::cell::RefCell;
use std::{env, fs};
use std::sync::{Arc, Mutex};
use mlua::{ExternalResult, Function, Lua, LuaSerdeExt, MultiValue, Result, Table, Value};
use mlua::prelude::{LuaError, LuaResult, LuaValue};
use reqwest;


pub  fn create_lua() -> Result<(Lua, Arc<Mutex<String>>)> {
    let lua = Lua::new();

    // 创建一个用于存储输出的字符串容器
    let output = Arc::new(Mutex::new(String::new()));
    let output_clone = output.clone();

    // 重定义 print 函数
    let print_override = lua.create_function(move |_, args: mlua::MultiValue| {
        let mut output = output_clone.lock().unwrap();
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                output.push_str("\t");  // print 默认使用 tab 分隔参数
            }

            if let mlua::Value::String(ss)= arg{
                output.push_str(ss.to_str()?.to_string().as_str());
            }else{
                output.push_str(&format!("{arg:#?}"));
            }

        }
        output.push_str("\n");  // print 默认在末尾添加换行符
        Ok(())
    })?;
    let to_string = lua.create_function(move |_, args: mlua::MultiValue| {
        let mut output = String::new();
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                output.push_str("\t");  // print 默认使用 tab 分隔参数
            }

            if let mlua::Value::String(ss)= arg{
                output.push_str(ss.to_str()?.to_string().as_str());
            }else{
                output.push_str(&format!("{arg:#?}"));
            }

        }
        output.push_str("\n");  // print 默认在末尾添加换行符
        Ok(output)
    })?;


    // 重定义 require 函数
    let require_override = lua.create_async_function(|lua, lua_file: String| async move  {
        let package: Table = lua.globals().get("package")?;
        let loaded: Table = package.get("loaded")?;
        if !loaded.contains_key(lua_file.as_str())?{
            //load from pages
            let uri =format!("/pages/{}",&lua_file );
            let lua_code: String  = lua.globals().get::<Table>("http")?.get::<Function>("get_text")?.call_async(uri.as_str()).await?;
            let lua_table: Table  =  lua.load(&lua_code).eval()?;
            loaded.set(lua_file.as_str(), lua_table)?;
        }
        loaded.get::<Table>(lua_file.as_str())

    })?;

    let get_json = lua.create_async_function(|lua, uri: String| async move {
        let mut uri = uri;
        if uri.starts_with("/"){
            uri  = format!("{}{}", env::var("HOST").unwrap(), uri);
        }

        let resp = reqwest::get(&uri)
            .await
            .and_then(|resp| resp.error_for_status())
            .into_lua_err()?;
        let json = resp.json::<serde_json::Value>().await.into_lua_err()?;
        lua.to_value(&json)
    })?;
    let get_text = lua.create_async_function(|lua, uri: String| async move {
        let mut uri = uri;
        if uri.starts_with("/"){
            uri  = format!("{}{}", env::var("HOST").unwrap(), uri);
        }
        let resp = reqwest::get(&uri)
            .await
            .and_then(|resp| resp.error_for_status())
            .into_lua_err()?;
        let json = resp.text().await.into_lua_err()?;
        lua.to_value(&json)
    })?;


    // 创建HTTP模块
    let http_module = lua.create_table()?;
    http_module.set("get_json", get_json)?;
    http_module.set("get_text", get_text)?;

    // 设置全局HTTP模块
    lua.globals().set("http", http_module)?;
    lua.globals().set("print", print_override)?;
    lua.globals().set("to_string", to_string)?;
    lua.globals().set("require", require_override)?;


    Ok((lua, output))
}
pub async fn run_lua(lua_code: &str) -> Result<String> {
    let (lua,output) = create_lua()?;

    lua.load(lua_code).exec_async().await?;

    //print log
    let output_log = output.lock().unwrap();

    Ok(format!("{output_log}"))
}
// 将 serde_json::Value 转换为 mlua::Value
fn json_to_lua_value(lua: &Lua, json: &serde_json::Value) -> Result<Value> {
    match json {
        serde_json::Value::Null => Ok(Value::Nil),
        serde_json::Value::Bool(b) => Ok(Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Number(f))
            } else {
                Ok(Value::Nil)
            }
        },
        serde_json::Value::String(s) => Ok(Value::String(lua.create_string(s)?)),
        serde_json::Value::Array(a) => {
            let table = lua.create_table()?;
            for (i, val) in a.iter().enumerate() {
                table.set(i + 1, json_to_lua_value(lua, val)?)?;  // Lua 索引从 1 开始
            }
            Ok(Value::Table(table))
        },
        serde_json::Value::Object(o) => {
            let table = lua.create_table()?;
            for (k, v) in o.iter() {
                table.set(k.clone(), json_to_lua_value(lua, v)?)?;
            }
            Ok(Value::Table(table))
        },
    }
}
pub async  fn lua_render(tpl_code: &str, data: serde_json::Value) -> Result<String> {
    let (lua,output) = create_lua()?;

    // Get the package.loaded table
    let package: Table = lua.globals().get("package")?;
    let loaded: Table = package.get("loaded")?;

    let template_engine: Table = lua.load(include_str!("template_engine.lua")).eval()?;

    // Register the module under the name "template_engine"
    loaded.set("template_engine", template_engine)?;

    let lua_table = json_to_lua_value(&lua, &data)?;

    lua.globals().set("_param", tpl_code)?;
    lua.globals().set("_data", lua_table)?;


    let lua_code = r#"
        local template_engine = require("template_engine")
        return template_engine.render(_param, _data)
        "#;

    let (success, result) :(bool,LuaValue) = lua.load(lua_code).eval_async().await?;
    if success{
        Ok(result.as_string().unwrap().to_string_lossy().to_string())
    }else{
        if result.is_error(){
            let error = result.as_error().unwrap();
            Err(error.clone())
        }else{
            Err(LuaError::external(format!("{:?}", result)))
        }
      
    }
    

}
#[cfg(test)]
mod tests {
    use std::env;
    use std::env::set_var;
    use serde_json::json;
    use play_shared::constants::DATA_DIR;
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let lua_code = r#"
        -- 简单GET请求
        local function test_get()
            print("执行GET请求...")
            local response = http.get_json("https://httpbin.org/anything?arg0=val0")
            print(response)
            local response = http.get_text("https://zhouzhipeng.com")
            print(response)

        end

        -- 执行测试
        test_get()
        print("\n所有HTTP请求测试完成!")
    "#;

        let output = run_lua(lua_code).await.unwrap();
        println!("{}", output);
    }
    #[tokio::test]
    async fn test_render() {


        let output = lua_render(r#"
         %htmlutils = require("html_utils2.lua")

        % a=1
        Hello,{{name}} {{a}}
        %local response = http.get_json("https://httpbin.org/anything?arg0=val0")
        {{response}}
        aa
        "#, json!({"name":"zhou"})).await.unwrap();

        println!("{}", output);
    }
    #[tokio::test]
    async fn test_require() {

        let output = run_lua(r#"
            htmlutils = require("html_utils.lua")
            local escaped = htmlutils.escape("<Hello & World>")
            print(escaped)
            print(htmlutils.unescape(escaped))
        "#).await;

        println!("{:?}", output);
    }
}
