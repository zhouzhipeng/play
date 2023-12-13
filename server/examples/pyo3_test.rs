
use std::path::Path;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

fn main() -> PyResult<()> {
    let path = Path::new("/Users/zhouzhipeng/RustroverProjects/play/server/python");
    // let py_app = fs::read_to_string(path.join("run_template.py"))?;
    let from_python = Python::with_gil(|py| -> PyResult<Py<PyAny>> {
        let syspath: &PyList = py.import("sys")?.getattr("path")?.downcast()?;
        syspath.insert(0, &path)?;
        let app: Py<PyAny> = py.import("simple_template")?
            .getattr("render_tpl")?
            .into();
        let data_dict = PyDict::new(py);
        data_dict.set_item("name", "zhouzhipeng")?;
        let args = ("hello {{name}}", "test.html",data_dict );
        app.call1(py, args)
    });

    println!("py: {}", from_python?);
    Ok(())
}
