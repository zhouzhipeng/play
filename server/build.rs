use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::path::{Path, PathBuf};

use pyo3::{Py, PyAny, PyResult, Python};
use pyo3::prelude::PyModule;
use pyoxidizerlib::environment::Environment;
use pyoxidizerlib::projectmgmt;
use serde_json::json;
use walkdir::WalkDir;
use zip_archive::Archiver;

use shared::utils::parse_create_sql;

const HOOKS_PATH: &str = "../.git/hooks";
const PRE_COMMIT_HOOK: &str = "#!/bin/sh
exec cargo test
";


fn main() {

    //test code.
    // test();


    //generate git pre-commit file.
    #[cfg(feature = "debug")]
    gen_pre_commit();


    //check if you forgot to add your new rust file into mod.rs
    #[cfg(feature = "debug")]
    check_mod_files();

    //generate python artifacts
    #[cfg(feature = "use_embed_python")]
    build_python_artifacts();

    #[cfg(feature = "debug")]
    gen_db_models_code();


    //generate rustc args.
    println!("cargo:rerun-if-changed=templates");
    println!("cargo:rerun-if-changed=static");
    println!("cargo:rerun-if-changed=config");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=python");
    println!("cargo:rerun-if-changed=doc");
    println!("cargo:rerun-if-changed=build.rs");

    let env = if cfg!(feature = "dev") { "dev" } else if cfg!(feature = "prod") { "prod" } else { "dev" };

    // let env = option_env!("ENV").unwrap_or("dev");
    println!("cargo:rustc-cfg=ENV=\"{}\"", env);
    println!("cargo:rustc-env=ENV={}", env);


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

                mod_contents  = mod_contents.replace(&expected, "");

                if file_stem.ends_with("_controller"){
                    if mod_contents.rfind(&file_stem).is_none(){
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

fn build_python_artifacts() {
    // let target_triple = current_platform::CURRENT_PLATFORM;
    // let flavor = "standalone";
    // let python_version = None; //default is 3.10
    let dest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("python/build");
    //
    // projectmgmt::generate_python_embedding_artifacts(
    //     &Environment::new().unwrap(),
    //     target_triple,
    //     flavor,
    //     python_version,
    //     &dest_path,
    // ).expect("build python artifacts failed.");

    //remove 'sqlite3' in config file.
    let match_str = "extra_build_script_line=cargo:rustc-link-lib=static=sqlite3\n";
    let config_path = dest_path.join("pyo3-build-config-file.txt");
    let config_file= fs::read_to_string(&config_path).unwrap();
    if config_file.contains(match_str){
        let config_file = config_file.replace(match_str, "");
        fs::write(config_path, config_file).unwrap();
    }

    //find a folder begin with 'config-'
    let stdlib_dir = dest_path.join("stdlib");

    for path in fs::read_dir(&stdlib_dir).unwrap(){
        let path = path.unwrap().path();
        if path.is_dir() && path.file_name().unwrap().to_str().unwrap().starts_with("config-"){
            let _ = fs::remove_dir_all(path);
        }
    }

    //delete USELESS directories.
    for p in [
        "test",
        "sqlite3",
        "tkinter",
        "pydoc_data",
        "asyncio",
        "concurrent",
        "xmlrpc",
        "xml",
        "unittest",
        "site-packages",
        "multiprocessing",
        "lib2to3",
        "turtledemo",
        "logging",
        "wsgiref",
        "idlelib",
        "venv",
        "importlib",
        "__pycache__",
        "email",
        "distutils",
        "dbm",
        "urllib",
        "turtle.py",
        "doctest.py",
        "tarfile.py",
        "ctypes",
        "ensurepip",
        "html",
        "http",
        "lib-dynload",
        "zoneinfo",
    ] {
        let file_or_dir = stdlib_dir.join(p);
        if file_or_dir.is_dir(){
            let _ = fs::remove_dir_all(file_or_dir);
        }else{
            fs::remove_file(file_or_dir);
        }


    }


    // sleep(Duration::from_secs(3));
    // set_var("PYO3_CONFIG_FILE","/Users/zhouzhipeng/RustroverProjects/play/server/python/build/pyo3-build-config-file.txt");
    // println!("cargo:rustc-env=PYO3_CONFIG_FILE={}","/Users/zhouzhipeng/RustroverProjects/play/server/python/build/pyo3-build-config-file.txt" );

    compress_directory(&dest_path.join("stdlib"), &dest_path);
}

fn compress_directory(dir: &PathBuf, zip_file: &PathBuf) {
    let origin = dir;
    let dest = zip_file;
    let thread_count = 4;

    let mut archiver = Archiver::new();
    archiver.push(origin);
    archiver.set_destination(dest);
    archiver.set_thread_count(thread_count);

    archiver.archive().expect("compress error!");
}

fn gen_db_models_code(){

    let sql = include_str!("doc/db_sqlite.sql");
    let table_info = parse_create_sql(sql);
    let ss = format!("table infos >> {:?}", table_info);

    pyo3::prepare_freethreaded_python();
    let py_app = include_str!("python/simple_template.py");
    let model_template = include_str!("doc/model_template.txt");

    Python::with_gil(|py| -> PyResult<Py<PyAny>> {
        // let syspath: &PyList = py.import("sys")?.getattr("path")?.downcast()?;
        // syspath.insert(0, &path)?;
        let render_fn: Py<PyAny> = PyModule::from_code(py, py_app, "", "")?
            .getattr("render_tpl_with_str_args")?
            .into();
        let set_debug_mode_fn: Py<PyAny> = PyModule::from_code(py, py_app, "", "")?
            .getattr("set_debug_mode")?
            .into();
        set_debug_mode_fn.call1(py, (true,))?;

        for info in table_info{
            let args = (model_template, "<tmp>", json!({"table_info": info}).to_string(), false);
            let r = render_fn.call1(py, args)?.to_string();
            let dest_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tables").join(format!("{}.rs", info.table_name));
            let mod_rs = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/tables/mod.rs");
            if !dest_file.exists(){
                fs::write(&dest_file, r).expect(format!("create file failed! : {:?}", &dest_file).as_str());

                //add to mod.rs
                let mut mod_rs_content = fs::read_to_string(&mod_rs).expect("read tables/mod.rs failed!");
                if !mod_rs_content.contains(format!("mod {}", info.table_name ).as_str() ){
                    mod_rs_content = format!("{}\npub mod {};", mod_rs_content, info.table_name);
                    fs::write(&mod_rs, mod_rs_content).expect("write tables/mod.rs failed");
                }
            }

        }

        Ok(render_fn)
    }).expect("run python error!");


}