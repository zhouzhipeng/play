use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use rustpython_vm as vm;
use rustpython_vm::convert::IntoObject;
use rustpython_vm::Interpreter;

fn main() {
    let html = fs::read_to_string("/Users/zhouzhipeng/RustroverProjects/play/templates/index.html").unwrap();

    let args = HashMap::from([
        ("name", Value::Str("周志鹏sss".into())),
        ("age", Value::Int(20)),
        ("male", Value::Bool(true))
    ]);

    match run_py_template(html.as_str(), "hello", args) {
        Ok(s) => println!("{}", s),
        Err(s) => println!("{}", s),
    }

    match run_py_code("print(1+1)") {
        Ok(s) => println!("{}", "execute ok "),
        Err(s) => println!("{}", s),
    }
}

pub enum Value {
    Str(String),
    Int(i64),
    Bool(bool),
}

fn run_py_template(source: &str, filename: &str, args: HashMap<&str, Value>) -> Result<String, String> {
    let interp = init_py_interpreter();
    interp.enter(|vm| {
        let scope = vm.new_scope_with_builtins();

        let module = vm::py_compile!(file = "python/simple_template.py");


        //mandatory args
        scope.locals.set_item("__source__", vm.ctx.new_str(source).into_object(), vm).unwrap();
        scope.locals.set_item("__filename__", vm.ctx.new_str(filename).into_object(), vm).unwrap();

        //custom args
        for (k, v) in args {
            match v {
                Value::Str(s) => scope.locals.set_item(k, vm.ctx.new_str(s).into_object(), vm).unwrap(),
                Value::Int(s) => scope.locals.set_item(k, vm.ctx.new_int(s).into_object(), vm).unwrap(),
                Value::Bool(s) => scope.locals.set_item(k, vm.ctx.new_bool(s).into_object(), vm).unwrap(),
            }
        }


        let res = vm.run_code_obj(vm.ctx.new_code(module), scope.clone());
        if let Err(exc) = res {
            let mut s = String::new();
            vm.write_exception_inner(&mut s, &exc).expect("write error");
            Err(s)
        } else {
            let result = scope.locals.get_item("__ret__", vm).unwrap().try_into_value::<String>(vm).unwrap();
            Ok(result)
        }
    })
}

fn init_py_interpreter() -> Interpreter {
    // extra embed python stdlib zip file to a directory and add it to syspath.
    let output_dir = "output_dir";
    let target_dir = PathBuf::from(output_dir); // Doesn't need to exist

    if !target_dir.exists() {
        println!("output_dir not existed , ready to extract stdlib to it.");

        let data = include_bytes!("../python/Lib.zip");

        let archive = Cursor::new(data);

        // The third parameter allows you to strip away toplevel directories.
        // If `archive` contained a single folder, that folder's contents would be extracted instead.
        zip_extract::extract(archive, &target_dir, true).unwrap();
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
    let interp = init_py_interpreter();
    interp.enter(|vm| {
        let scope = vm.new_scope_with_builtins();
        return match vm.run_code_string(scope, source, "<tmp>".into()) {
            Ok(s) => {
                // vm.unwrap_pyresult(s.to_pyresult(vm));
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