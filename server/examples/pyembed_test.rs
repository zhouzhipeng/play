use std::env::set_var;
use std::time::Instant;
use play::file_path;
use pyo3::{
    prelude::*,
    types::{PyBytes, PyDict},
};

include!(env!("DEFAULT_PYTHON_CONFIG_RS"));
fn main() {
    // set_var("PYO3_CONFIG_FILE",file_path!("/build/aarch64-apple-darwin/debug/resources/pyo3-build-config-file.txt"));
    for i in 0..100{

    // Get config from default_python_config.rs.
    // let config = default_python_config();
    let mut config = default_python_config();
    // config.filesystem_importer = true;
    // config.sys_paths.push("/path/to/python/standard/library");

    let interp = pyembed::MainPythonInterpreter::new(config).unwrap();
    // let start = Instant::now();
    // `py` is a `pyo3::Python` instance.
        let start = Instant::now();
    interp.with_gil(|py| {
        // let locals = PyDict::new(py);
        // locals.set_item("__args__","{}");
        // locals.set_item("__source__","123");
        // locals.set_item("__filename__","test");
        //
        // PyModule::from_code(py,include_str!("../../pyembedded/stdlib/simple_template.py"), "simple_template.py", "simple_template" );
        // py.run(include_str!("../../pyembedded/stdlib/run_template.py"), None, Some(locals)).unwrap();
        //
        // let ret = locals.get_item("__ret__").unwrap();
        py.run("import json",None, None, ).unwrap();
        // println!("ret >> {:?}", ret);
    });

    println!("used : {}ms", start.elapsed().as_millis());
    }
}