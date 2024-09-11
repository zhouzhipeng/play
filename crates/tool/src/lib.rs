use std::env::set_var;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use pyoxidizerlib::environment::Environment;
use pyoxidizerlib::projectmgmt;
use zip_archive::Archiver;

pub fn build_python_artifacts()->anyhow::Result<()> {
    let target_triple = current_platform::CURRENT_PLATFORM;
    let flavor = "standalone";
    let python_version = None; //default is 3.10
    let dest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../play_py_tpl/python/build");
    //
    projectmgmt::generate_python_embedding_artifacts(
        &Environment::new()?,
        target_triple,
        flavor,
        python_version,
        &dest_path,
    ).expect("build python artifacts failed.");

    //remove 'sqlite3' in config file.
    let match_str = "extra_build_script_line=cargo:rustc-link-lib=static=sqlite3\n";
    let config_path = dest_path.join("pyo3-build-config-file.txt");
    let config_file = fs::read_to_string(&config_path)?;
    if config_file.contains(match_str) {
        let config_file = config_file.replace(match_str, "");
        fs::write(config_path, config_file)?;
    }

    //find a folder begin with 'config-'
    let stdlib_dir = dest_path.join("stdlib");

    for path in fs::read_dir(&stdlib_dir)? {
        let path = path?.path();
        if path.is_dir() && path.file_name().unwrap().to_str().unwrap().starts_with("config-") {
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
        "http",
        "lib-dynload",
        "zoneinfo",
    ] {
        let file_or_dir = stdlib_dir.join(p);
        if file_or_dir.is_dir() {
            let _ = fs::remove_dir_all(file_or_dir);
        } else {
            fs::remove_file(file_or_dir);
        }
    }


    // sleep(Duration::from_secs(3));
    // set_var("PYO3_CONFIG_FILE","/Users/zhouzhipeng/RustroverProjects/play/server/python/build/pyo3-build-config-file.txt");
    // println!("cargo:rustc-env=PYO3_CONFIG_FILE={}","/Users/zhouzhipeng/RustroverProjects/play/server/python/build/pyo3-build-config-file.txt" );

    compress_directory(&dest_path.join("stdlib"), &dest_path);


    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_str().unwrap();
    println!("workspace root : {}", root);
    set_var("PYO3_CONFIG_FILE", format!("{}/crates/play_py_tpl/python/build/pyo3-build-config-file.txt", root));


    Ok(())
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

    println!("Done.")
}


pub fn build_dev(features: &str)->anyhow::Result<()>{
    build_python_artifacts()?;

    Command::new("cargo")
        .args(["build","--package", "play","--release", &format!("--features={}",features)])
        .spawn()?.wait()?;


    Ok(())
}