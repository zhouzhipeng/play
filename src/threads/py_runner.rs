use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender};
use rustpython_vm as vm;
use rustpython_vm::{Interpreter, py_compile, VirtualMachine};
use rustpython_vm::convert::IntoObject;
use tracing::{debug, info};

use crate::TemplateData;

fn run_py_template(vm: &VirtualMachine, template: String, filename: String, json_str_args: String) -> Result<String, String> {


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


fn init_py_interpreter() -> Interpreter {
    // extra embed python stdlib zip file to a directory and add it to syspath.
    let output_dir = "output_dir";
    let target_dir = PathBuf::from(output_dir); // Doesn't need to exist

    if !target_dir.exists() {
        info!("output_dir not existed , ready to extract stdlib to it.");

        let data = include_bytes!("../../python/Lib.zip");
        let archive = Cursor::new(data);

        // The third parameter allows you to strip away toplevel directories.
        // If `archive` contained a single folder, that folder's contents would be extracted instead.
        zip_extract::extract(archive, &target_dir, true).unwrap();

        //copy custom python files to output dir.
        let data = include_bytes!("../../python/simple_template.py");
        fs::write(Path::new(output_dir).join("simple_template.py"), data).expect("write file error!");
    }


    let mut settings = vm::Settings::default();
    settings.path_list.push(output_dir.to_string());
    Interpreter::with_init(settings, |vm| {
        // vm.add_native_modules(rustpython_stdlib::get_module_inits());
        // vm.insert_sys_path(vm.new_pyobj("/Users/zhouzhipeng/RustroverProjects/play/python"))
        //     .expect("add path");
        // let module = vm.import("simple_template", None, 0).unwrap();
        // let init_fn = module.get_attr("render_tpl", vm).unwrap();
        // init_fn.call(("hello {{name}}".to_string(),), vm).unwrap();
    })
}


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

pub async fn run(req_receiver: Receiver<TemplateData>, res_sender: Sender<String>) {
    let interpreter = init_py_interpreter();

    interpreter.enter(|vm| {
        // let module = vm::py_compile!(file = "python/test.py");
        // let code = vm.ctx.new_code(module);

        loop {
            // Receive the message from the channel.
            let data = req_receiver.recv().unwrap();


            let start = Instant::now();

            let r = match run_py_template(vm, data.template, data.filename, data.args.to_string()) {
                Ok(s) => s,
                Err(s) => s,
            };

            res_sender.try_send(r).expect("send error");
            info!("send spent:{}", start.elapsed().as_millis());
        }
    });
}