use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Instant;

use async_channel::{Receiver, Sender};
use futures::executor::block_on;
use rustpython_vm;
use rustpython_vm::{Interpreter, py_compile, VirtualMachine};
use rustpython_vm::convert::IntoObject;
use tracing::{info, warn};

use crate::TemplateData;

fn run_py_template(vm: &VirtualMachine, filename: &str, template: &str, json_str_args: String) -> Result<String, String> {


    // interpreter.enter(|vm| {
    let start = Instant::now();
    let scope = vm.new_scope_with_builtins();


    //mandatory args
    scope.locals.set_item("__source__", vm.ctx.new_str(template).into_object(), vm).unwrap();
    scope.locals.set_item("__filename__", vm.ctx.new_str(filename).into_object(), vm).unwrap();

    //custom args
    scope.locals.set_item("__args__", vm.ctx.new_str(json_str_args).into_object(), vm).unwrap();


    let res = vm.run_code_obj(vm.ctx.new_code(py_compile!(file = "python/run_template.py")), scope.clone());
    info!("scope spent:{}", start.elapsed().as_millis());
    if let Err(exc) = res {
        let mut s = String::new();
        vm.write_exception_inner(&mut s, &exc).expect("write error");
        Err(s)
    } else {
        let result = scope.locals.get_item("__ret__", vm).unwrap().try_into_value::<String>(vm).unwrap();
        Ok(result)
    }
}

macro_rules! include_py {
    ($t:literal) => {
        include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/python/", $t))
    };
}

macro_rules! copy_single_py {
    ($d: ident, $t:literal) => {
        let data =include_py!($t);
        fs::write($d.join($t), data).expect("write file error!");
    };
}

macro_rules! copy_zip_py {
    ($d: ident, $t:literal) => {
       let data = include_py!($t);
        // let data = include_bytes!("../../python/Lib.zip");
        let archive = Cursor::new(data);
        zip_extract::extract(archive, &$d, true).unwrap();
    };
}




fn init_py_interpreter() -> Interpreter {
    // extra embed python stdlib zip file to a directory and add it to syspath.
    let output_dir = "output_dir/python";
    let target_dir = PathBuf::from(output_dir); // Doesn't need to exist

    if !target_dir.exists() {
        info!("output_dir not existed , ready to extract stdlib to it.");

        let _ = fs::create_dir("output_dir");

        //python stdlib
        copy_zip_py!(target_dir,"Lib.zip");

        //copy single python files
        copy_single_py!(target_dir, "simple_template.py");
    }


    let mut settings = rustpython_vm::Settings::default();
    settings.path_list.push(output_dir.to_string());
    Interpreter::with_init(settings, |_vm| {
        // vm.add_native_modules(rustpython_stdlib::get_module_inits());
        // vm.insert_sys_path(vm.new_pyobj("/Users/zhouzhipeng/RustroverProjects/play/python"))
        //     .expect("add path");
        // let module = vm.import("simple_template", None, 0).unwrap();
        // let init_fn = module.get_attr("render_tpl", vm).unwrap();
        // init_fn.call(("hello {{name}}".to_string(),), vm).unwrap();
    })
}


#[allow(dead_code)]
fn run_py_code(source: &str) -> Result<(), String> {
    let start = Instant::now();
    let interp = init_py_interpreter();
    let elapsed = start.elapsed();
    info!("init spent: {}", elapsed.as_millis());

    let start = Instant::now();
    interp.enter(|vm| {
        let scope = vm.new_scope_with_builtins();
        return match vm.run_code_string(scope, source, "<tmp>".into()) {
            Ok(_) => {
                // vm.unwrap_pyresult(s.to_pyresult(vm));
                let elapsed = start.elapsed();
                info!("run_code_string spent: {}", elapsed.as_millis());
                Ok(())
            }
            Err(exc) => {
                let mut s = String::new();
                vm.write_exception_inner(&mut s, &exc).expect("write error");
                Err(s)
            }
        };
    })
}


pub  fn run(req_receiver: Receiver<TemplateData>, res_sender: Sender<String>) {
    info!("py_runner start...");
    let interpreter = init_py_interpreter();
    interpreter.enter(|vm| {
        block_on(async move{
            loop {
                info!("ready to listen for template render request in py_runner...");
                // Receive the message from the channel.
                let data = match req_receiver.recv().await {
                    Ok(s) => s,
                    Err(e) => {
                        warn!("req_receiver.recv error : {}", e);
                        return;
                    }
                };


                let r = match run_py_template(&vm, data.template.name, data.template.content, data.args.to_string()) {
                    Ok(s) => s,
                    Err(s) => s,
                };

                res_sender.send(r).await.expect("send error");
            }
        });
    });



}