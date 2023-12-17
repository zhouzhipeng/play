use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::prelude::*;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use fs_extra::dir::CopyOptions;
use pyo3::{Py, PyAny, PyResult, Python};
use pyo3::prelude::PyModule;
use pyoxidizerlib::environment::Environment;
use pyoxidizerlib::projectmgmt;
use serde_json::json;
use walkdir::WalkDir;
use wasm_pack::command::build::{BuildOptions, Target};
use wasm_pack::command::run_wasm_pack;
use zip_archive::Archiver;
use shared::utils::parse_create_sql;


const HOOKS_PATH: &str = "../.git/hooks";
const PRE_COMMIT_HOOK: &str = "#!/bin/sh
exec cargo test
";


fn main() {
    if Ok("release".to_owned()) != env::var("PROFILE") {

        //test code.
        test();

        //generate git pre-commit file.
        gen_pre_commit();


        //check if you forgot to add your new rust file into mod.rs
        check_mod_files();


        //will trigger deadlock when build --release
        //copy wasm files from `client` crate
        copy_wasm_files();

        //generate python artifacts
        #[cfg(feature = "use_embed_python")]
        build_python_artifacts();

        #[cfg(feature = "debug")]
        gen_db_models_code();
    }


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


fn copy_wasm_files() {
    let client_path = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("wasm");
    // let _ = fs::remove_dir_all(client_path.join("pkg"));
    run_wasm_pack(wasm_pack::command::Command::Build(BuildOptions {
        path: Some(client_path),
        out_dir: "pkg".to_string(),
        release: true,
        target: Target::Web,
        #[cfg(feature = "dev")]
        extra_options: vec! {"--features".to_string(), "console_error_panic_hook".to_string()},
        ..BuildOptions::default()
    })).expect("wasm-pack failed");

    let from_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join(Path::new("wasm/pkg"));
    fs_extra::dir::copy(from_dir, Path::new(env!("CARGO_MANIFEST_DIR")).join("static/wasm"), &CopyOptions::new().overwrite(true)).expect("copy wasm files failed!");
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
        .mode(0o755)
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
    let target_triple = current_platform::CURRENT_PLATFORM;
    let flavor = "standalone";
    let python_version = "3.10";
    let dest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("python/build");

    projectmgmt::generate_python_embedding_artifacts(
        &Environment::new().unwrap(),
        target_triple,
        flavor,
        Some(python_version),
        &dest_path,
    ).expect("build python artifacts failed.");

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
        "config-3.10-darwin",
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
        let _ = fs::remove_dir_all(dest_path.join("stdlib").join(p));
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