use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::path::Path;
//
// use pyo3::{Py, PyAny, PyResult, Python};
// use pyo3::prelude::PyModule;
use serde_json::json;
use walkdir::WalkDir;

use shared::utils::parse_create_sql;

const HOOKS_PATH: &str = "../.git/hooks";
const PRE_COMMIT_HOOK: &str = "#!/bin/sh
exec cargo test
";


fn main() {
    //generate git pre-commit file.
    #[cfg(feature = "debug")]
    gen_pre_commit();

    //check if you forgot to add your new rust file into mod.rs
    #[cfg(feature = "debug")]
    check_mod_files();

    // #[cfg(feature = "debug")]
    // gen_db_models_code();


    //generate rustc args.
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed=python");
    println!("cargo:rerun-if-changed=doc");

}


fn check_mod_files() {
    let walker = WalkDir::new("src").into_iter();

    for entry in walker {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".rs") && name != "mod.rs" && name != "lib.rs" && name != "main.rs" {
            let file_stem = Path::new(&name).file_stem().unwrap().to_str().unwrap();
            let parent_mod_file = entry.path().parent().unwrap().join("mod.rs");
            if parent_mod_file.exists() {
                let mut f = File::open(&parent_mod_file).unwrap();
                let mut mod_contents = String::new();
                f.read_to_string(&mut mod_contents).unwrap();
                let expected = format!("mod {};", file_stem);
                if !mod_contents.contains(&expected) {
                    panic!("Error >> File {}.rs is not included in mod.rs", file_stem);
                }

                mod_contents = mod_contents.replace(&expected, "");

                if file_stem.ends_with("_controller") {
                    if mod_contents.rfind(&file_stem).is_none() {
                        panic!("Error >> you need to register controller: {}  in controller/mod.rs", file_stem);
                    }
                }
            }
        }
    }
}

fn gen_pre_commit() {
    fs::create_dir_all(HOOKS_PATH).unwrap();

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        // .mode(0o755)
        .open(format!("{}/pre-commit", HOOKS_PATH))
        .unwrap();

    file.write_all(PRE_COMMIT_HOOK.as_bytes()).unwrap();
}

fn test() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("hello.rs");

    fs::write(
        &dest_path,
        "pub fn message() -> &'static str {
            \"Hello, World!\"
        }
        ",
    ).unwrap();
}


// fn gen_db_models_code() {
//     let sql = include_str!("../doc/db_sqlite.sql");
//     let table_info = parse_create_sql(sql);
//     let ss = format!("table infos >> {:?}", table_info);
//
//     pyo3::prepare_freethreaded_python();
//     let py_app = include_str!("python/simple_template.py");
//     let model_template = include_str!("../doc/tmpl/model_template.rs.txt");
//     let controller_template = include_str!("../doc/tmpl/controller_template.rs.txt");
//
//     Python::with_gil(|py| -> PyResult<Py<PyAny>> {
//         // let syspath: &PyList = py.import("sys")?.getattr("path")?.downcast()?;
//         // syspath.insert(0, &path)?;
//         let render_fn: Py<PyAny> = PyModule::from_code(py, py_app, "", "")?
//             .getattr("render_tpl_with_str_args")?
//             .into();
//         let set_debug_mode_fn: Py<PyAny> = PyModule::from_code(py, py_app, "", "")?
//             .getattr("set_debug_mode")?
//             .into();
//         set_debug_mode_fn.call1(py, (true, ))?;
//
//         for info in table_info {
//             let args = (model_template, "<tmp>", json!({"table_info": info}).to_string(), false);
//             let model_content = render_fn.call1(py, args)?.to_string();
//
//
//             let args = (controller_template, "<tmp>", json!({"table_name": info.table_name}).to_string(), false);
//             let controller_content = render_fn.call1(py, args)?.to_string();
//
//             let dest_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tables").join(format!("{}.rs", info.table_name));
//             let controller_dest_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/controller").join(format!("{}_controller.rs", info.table_name));
//             let mod_rs = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tables/mod.rs");
//             if !dest_file.exists() {
//                 fs::write(&dest_file, model_content).expect(format!("create file failed! : {:?}", &dest_file).as_str());
//
//                 //add to mod.rs
//                 let mut mod_rs_content = fs::read_to_string(&mod_rs).expect("read tables/mod.rs failed!");
//                 if !mod_rs_content.contains(format!("mod {}", info.table_name).as_str()) {
//                     mod_rs_content = format!("{}\npub mod {};", mod_rs_content, info.table_name);
//                     fs::write(&mod_rs, mod_rs_content).expect("write tables/mod.rs failed");
//                 }
//             }
//
//             if !controller_dest_file.exists(){
//                 fs::write(&controller_dest_file, controller_content).expect(format!("create file failed! : {:?}", &controller_dest_file).as_str());
//             }
//
//             let template_dir  = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("templates/{}", info.table_name));
//             if !template_dir.exists(){
//                 fs::create_dir(&template_dir).expect(&format!("create dir :{:?} failed", template_dir));
//                 let template = include_str!("../doc/tmpl/list_template.html.txt");
//                 fs::write(template_dir.join("list.html"), template).expect("create list.html failed!");
//
//             }
//
//
//
//         }
//
//         Ok(render_fn)
//     }).expect("run python error!");
// }