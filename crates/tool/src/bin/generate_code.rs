use std::fs;
use std::path::Path;
use pyo3::{Py, PyAny, PyResult, Python};
use pyo3::prelude::PyModule;
use shared::utils::{parse_create_sql, SQLiteDialect};
use serde_json::json;
use std::process::{Command, Output};

fn main() {
    match check_git_status() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !stdout.is_empty(){
                println!("Git Status Output:\n{}", stdout);

                if !stdout.contains("working tree clean"){
                    eprintln!("Err : pls commit your changes before run `cargo generate`");
                }else{
                    gen_db_models_code();
                    println!("Generated task done.");
                }
            }

            if !stderr.is_empty() {
                eprintln!("Git Error Output:\n{}", stderr);
            }
        }
        Err(e) => eprintln!("Failed to check git status: {}", e),
    }

}
fn check_git_status() -> Result<Output, std::io::Error> {
    Command::new("git")
        .arg("status")
        .output()
}

fn gen_db_models_code() {
    //


    let sql = include_str!("../../../doc/db_sqlite.sql");
    let table_info = parse_create_sql(sql,SQLiteDialect{});
    let ss = format!("table infos >> {:?}", table_info);

    pyo3::prepare_freethreaded_python();
    let py_app = include_str!("../../../play_py_tpl/python/simple_template.py");
    let model_template = include_str!("../../../doc/tmpl/model_template.rs.txt");
    let controller_template = include_str!("../../../doc/tmpl/controller_template.rs.txt");

    Python::with_gil(|py| -> PyResult<Py<PyAny>> {
        // let syspath: &PyList = py.import("sys")?.getattr("path")?.downcast()?;
        // syspath.insert(0, &path)?;
        let render_fn: Py<PyAny> = PyModule::from_code(py, py_app, "", "")?
            .getattr("render_tpl_with_str_args")?
            .into();
        let set_debug_mode_fn: Py<PyAny> = PyModule::from_code(py, py_app, "", "")?
            .getattr("set_debug_mode")?
            .into();
        set_debug_mode_fn.call1(py, (true, ))?;

        for info in table_info {
            let args = (model_template, "<tmp>", json!({"table_info": info}).to_string(), false);
            let model_content = render_fn.call1(py, args)?.to_string();


            let args = (controller_template, "<tmp>", json!({"table_name": info.table_name}).to_string(), false);
            let controller_content = render_fn.call1(py, args)?.to_string();

            let dest_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../server/src/tables").join(format!("{}.rs", info.table_name));
            let controller_dest_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../server/src/controller").join(format!("{}_controller.rs", info.table_name));
            let mod_rs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../server/src/tables/mod.rs");
            if !dest_file.exists() {
                fs::write(&dest_file, model_content).expect(format!("create file failed! : {:?}", &dest_file).as_str());

                //add to mod.rs
                let mut mod_rs_content = fs::read_to_string(&mod_rs).expect("read tables/mod.rs failed!");
                if !mod_rs_content.contains(format!("mod {};", info.table_name).as_str()) {
                    mod_rs_content = mod_rs_content.replace("//PLACEHOLDER:TABLE_MOD", &format!("\npub mod {};\n//PLACEHOLDER:TABLE_MOD\n", info.table_name));
                    fs::write(&mod_rs, mod_rs_content).expect("write tables/mod.rs failed");
                }



                if !controller_dest_file.exists(){
                    fs::write(&controller_dest_file, controller_content).expect(format!("create file failed! : {:?}", &controller_dest_file).as_str());


                    //add to mod.rs
                    let mod_rs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../server/src/controller/mod.rs");
                    let mut mod_rs_content = fs::read_to_string(&mod_rs).expect("read controller/mod.rs failed!");
                    if !mod_rs_content.contains(format!("mod {}_controller;", info.table_name).as_str()) {
                        mod_rs_content = mod_rs_content.replace("//PLACEHOLDER:CONTROLLER_MOD", &format!("\nmod {}_controller;\n//PLACEHOLDER:CONTROLLER_MOD\n", info.table_name));
                        mod_rs_content = mod_rs_content.replace("//PLACEHOLDER:CONTROLLER_REGISTER", &format!("{}_controller,\n//PLACEHOLDER:CONTROLLER_REGISTER\n", info.table_name));
                        fs::write(&mod_rs, mod_rs_content).expect("write controller/mod.rs failed");
                    }
                }

                let template_dir  = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../server/templates/{}", info.table_name));
                if !template_dir.exists(){
                    fs::create_dir(&template_dir).expect(&format!("create dir :{:?} failed", template_dir));
                    let template = include_str!("../../../doc/tmpl/list_template.html.txt");
                    fs::write(template_dir.join("list.html"), template).expect("create list.html failed!");

                }

            }



        }

        Ok(render_fn)
    }).expect("run python error!");
}