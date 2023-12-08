use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Instant;

use async_channel::{Receiver, Sender};
use futures::executor::block_on;
use rustpython_vm;
use rustpython_vm::{Interpreter, py_compile, VirtualMachine};
use rustpython_vm::convert::{IntoObject, ToPyObject, ToPyResult};
use tracing::{error, info, warn};

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

use rustpython_vm::{pymodule};



#[pymodule]
mod mymod {
    use std::collections::HashMap;
    use rustpython_vm::builtins::{PyListRef, PyStrRef};
    use regex::Regex;
    use rustpython_vm::{pymodule, PyObjectRef, PyResult, VirtualMachine};
    use rustpython_vm::convert::ToPyObject;
    use reqwest::blocking::Client;
    use shared::constants::HOST;

    #[pyfunction]
    fn request(s: PyStrRef) -> String {
        let client = Client::builder().build().unwrap();
        let res = client.get(HOST).send().unwrap();
        format!("{}, {}", s, res.text().unwrap())

    }

    #[pyfunction]
    fn do_thing(x: i32) -> i32 {
        x + 1
    }

    #[pyfunction]
    fn other_thing(s: PyStrRef) -> (String, usize) {
        let new_string = format!("hello from rust, {}!", s);
        let prev_len = s.as_str().len();
        (new_string, prev_len)
    }
    #[pyfunction]
    fn escape(s: PyStrRef) -> String {
        regex::escape(s.to_string().as_str())
    }
    #[pyfunction]
    fn search(pattern: PyStrRef, s: PyStrRef,vm: &VirtualMachine) -> Vec<PyObjectRef> {
        use fancy_regex::Regex;
        let re = Regex::new(&pattern.to_string()).unwrap();
        let hay =  &s.to_string();

        println!("pattern >> \n {} , \n   s >> \n {}", pattern.as_str(), s.as_str());


        let mut start = 0;
        let mut end=0;
        let mut groups = vec![];
        for mat in re.captures_iter(hay) {
            let mat = mat.unwrap();

            let mut i=0;
            for mat in mat.iter(){
                let mat = match mat{
                    None => continue,
                    Some(c) => c,
                };
                let g = mat.as_str();
                if i==0{
                    start = mat.start();
                    end = mat.end();
                }

                if mat.start() < start{
                    start = mat.start();
                }

                if mat.end() > end{
                    end = mat.end();
                }

                println!("search match  >> \n {}, start = {}, end={}",g, mat.start(), mat.end() );

                if i!=0{
                    groups.push(g);
                }

                i+=1;
            }




            break
        }


        if groups.len()>0{


            let results = (start, end, groups.join("||"));
            println!("search results >> {:?}", results);

            vec![vm.new_pyobj(results)]
        }else{
            vec![]
        }
    }
    #[pyfunction]
    fn find_all( pattern: PyStrRef, s: PyStrRef,vm: &VirtualMachine) -> Vec<PyObjectRef> {
        use fancy_regex::Regex;
        let re = Regex::new(&pattern.to_string()).unwrap();
        let hay =  &s.to_string();


        let mut results = vec![];
        for mat in re.captures_iter(hay) {
            let mat = mat.unwrap();
            let mut groups = vec![];
            let mut i=-1;
            let mut start = 0;
            let mut end=0;
            for mat in mat.iter(){
                i+=1;
                let mat = match mat{
                    None => continue,
                    Some(c) => c,
                };
                let g = mat.as_str();
                if i==0{
                    start = mat.start();
                    end = mat.end();
                    continue;
                }


                if mat.start() < start{
                    start = mat.start();
                }

                if mat.end() > end{
                    end = mat.end();
                }




                if i!=0{
                    groups.push(g);
                }


            }

            println!("find_all match  >> \n {:?}, start = {}, end={}",groups, start, end );

            results.push(vm.new_pyobj((start,end,groups.join("||"))));

        }



        println!("find_all results >> {:?}", results);
        results
    }
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
    Interpreter::with_init(settings, |vm| {
        vm.add_native_module("mymod".to_owned(), Box::new(mymod::make_module));

        // vm.add_native_modules(rustpython_stdlib::get_module_inits());
        // vm.insert_sys_path(vm.new_pyobj("/Users/zhouzhipeng/RustroverProjects/play/python"))
        //     .expect("add path");
        // let module = vm.import("simple_template", None, 0).unwrap();
        // let init_fn = module.get_attr("render_tpl", vm).unwrap();
        // init_fn.call(("hello {{name}}".to_string(),), vm).unwrap();
    })
}


fn run_py_code(source: &str) -> Result<String, String> {
    let start = Instant::now();
    let interp = init_py_interpreter();
    let elapsed = start.elapsed();
    info!("init spent: {}", elapsed.as_millis());

    let start = Instant::now();
    interp.enter(|vm| {
        let scope = vm.new_scope_with_builtins();
        return match vm.run_code_string(scope.clone(), source, "<tmp>".into()) {
            Ok(a) => {
                let result = scope.locals.get_item("__ret__", vm).unwrap().try_into_value::<String>(vm).unwrap();
                Ok(result)
            }
            Err(exc) => {
                let mut s = String::new();
                vm.write_exception_inner(&mut s, &exc).expect("write error");
                Err(s)
            }
        };
    })
}


pub fn run(req_receiver: Receiver<TemplateData>) {
    info!("py_runner start...");
    let interpreter = init_py_interpreter();
    interpreter.enter(|vm| {
        block_on(async move {
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

                if data.response.is_closed(){
                    warn!("response already closed , skip rendering");
                    continue
                }

                let r = match run_py_template(vm, data.template.name, data.template.content, data.args.to_string()) {
                    Ok(s) => s,
                    Err(s) => s,
                };

                if data.response.is_closed(){
                    warn!("response already closed , skip send back.");
                    continue
                }

                if let Err(e)=data.response.send(r).await{
                    error!("py_runner send error : {:?}", e.to_string() );
                }
            }
        });
    });
}


#[cfg(test)]
mod tests {
    use regex::Regex;
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[tokio::test]
    async fn test_all() -> anyhow::Result<()> {
        let result = run_py_code(r#"
import mymod

(r,_)=mymod.other_thing('hello')
r = r+ str(mymod.find_all('aa', 'aabb'))

locals()['__ret__']=r
        "#).expect("init error!");

        println!("result >>> {}", result);

        Ok(())
    }


    #[ignore]
    #[test]
    fn test_re()->anyhow::Result<()> {
        use fancy_regex::Regex;

        let pattern_str = r#"
(?m)\{\{((?:([urbURB]?(?:''(?!')|""(?!")|'{6}|"{6}|'(?:[^\\']|\\.)+?'|"(?:[^\\"]|\\.)+?"|'{3}(?:[^\\]|\\.)+?'{3}|"{3}(?:[^\\]|\\.)+?"{3}))|[^'"]*?)+)\}\}
        "#;



        let regex = Regex::new(pattern_str).unwrap();
        let input_text = "\
 <style>
  body{
    margin: auto;
  }
";


        // Perform the regex match
        if let Ok(mat) = regex.find(input_text) {
            let mat = mat.unwrap();

            println!("Match found: {}", input_text[mat.start()..mat.end()].to_string());
        } else {
            println!("No match found.");
        }
        Ok(())
    }

    #[test]
    fn test_re2(){
        let tokens = vec!["<%", "%>", "%", "{{", "}}"];
        for t in tokens{
            let escaped_text = regex::escape(t);
            println!("escaped_text :   {}", escaped_text);
        }



    }

    #[ignore]
    #[test]
    fn test_search(){
        use fancy_regex::Regex;
        let re = Regex::new(r#"(?m)([urbURB]?(?:''(?!')|""(?!")|'{6}|"{6}|'(?:[^\\']|\\.)+?'|"(?:[^\\"]|\\.)+?"|'{3}(?:[^\\]|\\.|\n)+?'{3}|"{3}(?:[^\\]|\\.|\n)+?"{3}))|(#.*)|([\[\{\(])|([\]\}\)])|^([ \t]*(?:if|for|while|with|try|def|class)\b)|^([ \t]*(?:elif|else|except|finally)\b)|((?:^|;)[ \t]*end[ \t]*(?=(?:%>[ \t]*)?\r?$|;|#))|(%>[ \t]*(?=\r?$))|(\r?\n)"#).unwrap();
        let hay =  r#"<ul>
    % for article in articles:
    <li>
        <a href="/article/{{article.id}}">{{article.title}}</a>
    </li>
    %end

    my name is :{{name}}
</ul>"#;




        let mut start = 0;
        let mut end=0;
        let mut groups = vec![];
        for mat in re.captures_iter(hay) {
            let mat = mat.unwrap();

            let mut i=0;
            let mut find=false;
            for mat in mat.iter(){
                let mat = match mat{
                    None => continue,
                    Some(c) => c,
                };
                let g = mat.as_str();
                if i==0{
                    start = mat.start();
                    end = mat.end();
                }

                if mat.start() < start{
                    start = mat.start();
                }

                if mat.end() > end{
                    end = mat.end();
                }

                println!("search match  >> \n {}, start = {}, end={}",g, mat.start(), mat.end() );

                if i!=0{
                    groups.push(g);
                }else{
                    find = true;
                }

                i+=1;
            }



            if find{
                break
            }

        }

        let results = (start, end, groups);

        println!("results >> {:?}", results);
    }


    #[ignore]
    #[test]
    fn test_finditer(){
        use fancy_regex::Regex;
        let re = Regex::new(r"(\w)haha(\w)").unwrap();
        let hay =  "aahahabbaahahabb";


        for mat in re.captures_iter(hay) {
            let mat = mat.unwrap();
            let mut groups = vec![];
            let mut i=-1;
            let mut start = 0;
            let mut end=0;
            for mat in mat.iter(){
                i+=1;
                let mat = mat.unwrap();
                let g = mat.as_str();
                if i==0{
                    start = mat.start();
                    end = mat.end();
                    continue;
                }


                if mat.start() < start{
                    start = mat.start();
                }

                if mat.end() > end{
                    end = mat.end();
                }




                if i!=0{
                    groups.push(g);
                }


            }

            println!("search match  >> \n {:?}, start = {}, end={}",groups, start, end );


        }

        let results = ();

        println!("results >> {:?}", results);
    }

}