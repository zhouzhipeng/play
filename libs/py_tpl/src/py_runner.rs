#![allow(warnings)]

use std::env::set_var;
use std::{env, fs, thread};

use std::io::Cursor;
use std::time::Duration;


use async_channel::Receiver;
use async_trait::async_trait;
use include_dir::{Dir, include_dir};
use lazy_static::lazy_static;
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use reqwest::blocking::Client;

use tracing::{error, info, warn};
use shared::constants::DATA_DIR;
use shared::file_path;

use shared::tpl_engine_api::{Template, TemplateData, TplEngineAPI};
use shared::utils::{GenericDialect, parse_create_sql};


macro_rules! include_py {
    ($t:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/python/", $t))
    };
}







#[pyfunction]
fn add_one(x: i64) -> i64 {
    x + 1
}
#[pyfunction]
fn parse_create_sql_str(sql: String) -> PyResult<String> {
    let info =parse_create_sql(&sql, GenericDialect{});
    Ok(serde_json::to_string(&info).unwrap())
}


#[pyfunction]
fn read_file(filename : String) -> PyResult<String> {
    // info!("read file {} from python call", filename);
    let mut filename = filename.clone();
    filename.remove(0);

    #[cfg(feature = "debug")]
    let c = fs::read_to_string(format!("{}/../../server/templates/{}", env!("CARGO_MANIFEST_DIR"),filename)).unwrap_or(format!("Error : file not found :{}",format!("{}/../../server/templates/{}", env!("CARGO_MANIFEST_DIR"),filename)));
    #[cfg(not(feature = "debug"))]
    let c = crate::TEMPLATES_DIR.get_file(filename).unwrap().contents_utf8().unwrap().to_string();
    // info!(" content  >> {}", c);
    Ok(c)
}
#[pyfunction]
fn http_get(url : String) -> PyResult<String> {
    // 使用 std::thread::spawn 来创建一个新线程
    let handle = thread::spawn(move || {
        let client = Client::builder().timeout(Duration::from_secs(3)).build().unwrap();
        // 尝试发送请求并获取响应
        match client.get(&url).send() {
            Ok(response) => {
                match response.text() {
                    Ok(parsed) => {
                        // 将响应数据转换为字符串（或任何适合你数据的格式）
                        Ok(parsed)
                    },
                    Err(_) => Err("Failed to parse response"),
                }
            },
            Err(e) => Err("Failed to send request"),
        }
    });

    // 等待线程结束并获取结果
    match handle.join() {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(e)) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e)),
        Err(_) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Thread panicked")),
    }

}

#[pymodule]
fn foo(_py: Python<'_>, foo_module: &PyModule) -> PyResult<()> {
    foo_module.add_function(wrap_pyfunction!(add_one, foo_module)?)?;
    foo_module.add_function(wrap_pyfunction!(read_file, foo_module)?)?;
    foo_module.add_function(wrap_pyfunction!(http_get, foo_module)?)?;
    foo_module.add_function(wrap_pyfunction!(parse_create_sql_str, foo_module)?)?;
    Ok(())
}



pub struct PyRunner{

}

#[async_trait]
impl TplEngineAPI for PyRunner{
    async fn run_loop(&self, req_receiver: Receiver<TemplateData>) {
        info!("py_runner start...");

        #[cfg(feature = "use_embed_python")]
        if option_env!("PYO3_CONFIG_FILE").is_some() {
            info!("use embed python!");
            let data_dir = env::var(DATA_DIR).unwrap();
            //decompress stdlib.zip to output_dir
            let data = include_bytes!(file_path!("/python/build/stdlib.zip"));
            let archive = Cursor::new(data);
            zip_extract::extract(archive, &data_dir.as_ref(), false).unwrap();
            set_var("PYTHONPATH", format!("{}/stdlib", data_dir));
            set_var("PYTHONHOME", &data_dir); //just to supress warning logs.
        }else{
            info!("PYO3_CONFIG_FILE not set , use system python as fallback!");
        }

        #[cfg(not(feature = "use_embed_python"))]
        info!("use system python!");


        //init
        pyo3::append_to_inittab!(foo);
        pyo3::prepare_freethreaded_python();


        // let path = Path::new(file_path!("/python"));
        // let py_app = fs::read_to_string(path.join("run_template.py"))?;
        let py_app = include_py!("simple_template.py");

        let py_render_fn = Python::with_gil(|py| -> PyResult<Py<PyAny>> {

            info!("python version  : {}.{}.{}", py.version_info().major,py.version_info().minor,py.version_info().patch,);
            // let syspath: &PyList = py.import("sys")?.getattr("path")?.downcast()?;
            // syspath.insert(0, &path)?;
            let render_fn: Py<PyAny> = PyModule::from_code(py, py_app, "simple_template.py", "simple_template")?
                .getattr("render_tpl_with_str_args")?
                .into();
            // let cache_template_fn: Py<PyAny> = PyModule::from_code(py, py_app, "", "")?
            //     .getattr("cache_template")?
            //     .into();
            let set_debug_mode_fn: Py<PyAny> = PyModule::from_code(py, py_app, "", "")?
                .getattr("set_debug_mode")?
                .into();
            set_debug_mode_fn.call1(py, (cfg!(feature = "debug"),))?;


            Ok(render_fn)
        }).expect("run python error!");


        loop {
            // info!("ready to listen for template render request in py_runner...");
            // Receive the message from the channel.
            let data = match req_receiver.recv().await {
                Ok(s) => s,
                Err(e) => {
                    if req_receiver.is_closed(){
                        error!("req_receiver channel closed, so exiting py_runner thread.");
                        break
                    }
                    warn!("req_receiver.recv error : {}", e);
                    continue;
                }
            };

            if data.response.is_closed() {
                warn!("response already closed , skip rendering");
                continue;
            }

            // let aa = [("name", "zhouzhipeng")];
            // aa[0].key();
            let (name, content, run_code, use_cache) = match data.template {
                Template::StaticTemplate { name, content } => (name.to_string(), content.to_string(), false, true),
                Template::DynamicTemplate { name, content } => (name, content, false, false),
                Template::PythonCode { name, content } =>(name, content, true, false)
            };



            let r = match Python::with_gil(|py| -> PyResult<String> {
                // let syspath: &PyList = py.import("sys")?.getattr("path")?.downcast()?;
                // syspath.insert(0, &path)?;
                // let app: Py<PyAny> =  PyModule::from_code(py, py_app, "", "")?
                //     .getattr("render_tpl_with_str_args")?
                //     .into();
                if run_code {
                    match py.run(&content, None, None){
                        Ok(_) => {}
                        Err(e) => {
                            // e.display(py);
                            return Ok(Self::gen_err_msg(py, e))

                        }
                    };
                    Ok("ok".to_string())
                } else {
                    let args = (&content, name, data.args.to_string(), use_cache);
                    let r = match py_render_fn.call1(py, args){
                        Ok(s) => s.to_string(),
                        Err(e) => {
                            // e.display(py);
                            return Ok(Self::gen_err_msg(py, e))
                        }
                    };
                    Ok(r)
                }
            }){
                Ok(s) => s,
                Err(e) => e.to_string(),
            };

            if data.response.is_closed() {
                warn!("response already closed , skip send back.");
                continue;
            }

            if let Err(e) = data.response.try_send(r) {
                error!("py_runner send error : {:?}", e.to_string() );
            }
        }
    }
}

impl PyRunner {
    fn gen_err_msg(py: Python, e: PyErr)->String{
        let r = if let Some(ss) = e.traceback(py) {
            let err_msg = format!("{}{}", ss.format().unwrap_or_default(), e);
            error!("python execution error >>\n  {}",err_msg);
             err_msg
        } else {
            let err_msg = format!("{}{}", "unknown error", e);
            error!("python execution error >> {}",err_msg);
             err_msg
        };



        format!("[ERROR]{}", r)
    }
}
