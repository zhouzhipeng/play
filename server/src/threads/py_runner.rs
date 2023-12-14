 #![allow(warnings)]
use std::env::set_var;

use std::io::Cursor;



use async_channel::Receiver;
use pyo3::prelude::*;

use tracing::{error, info, warn};

use crate::{file_path, TemplateData};
 use crate::controller::Template;


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

pub async fn run(req_receiver: Receiver<TemplateData>) -> ! {
    info!("py_runner start...");

    // set_var("PYO3_CONFIG_FILE","/Users/zhouzhipeng/RustroverProjects/play/server/python/build/pyo3-build-config-file.txt");

    if option_env!("PYO3_CONFIG_FILE").is_some(){
        //decompress stdlib.zip to output_dir
        let data = include_bytes!(file_path!("/python/build/stdlib.zip"));
        let archive = Cursor::new(data);
        zip_extract::extract(archive, "output_dir".as_ref(), false).unwrap();
        set_var("PYTHONPATH","output_dir/stdlib");
        set_var("PYTHONHOME","output_dir"); //just to supress warning logs.
    }


    pyo3::prepare_freethreaded_python();
    // let path = Path::new(file_path!("/python"));
    // let py_app = fs::read_to_string(path.join("run_template.py"))?;
    let py_app = include_str!(file_path!("/python/simple_template.py"));

    let py_render_fn = Python::with_gil(|py| -> PyResult<Py<PyAny>> {
        // let syspath: &PyList = py.import("sys")?.getattr("path")?.downcast()?;
        // syspath.insert(0, &path)?;
        let app: Py<PyAny> =  PyModule::from_code(py, py_app, "", "")?
            .getattr("render_tpl_with_str_args")?
            .into();
        Ok(app)
    }).expect("run python error!");



    loop {
        // info!("ready to listen for template render request in py_runner...");
        // Receive the message from the channel.
        let data = match req_receiver.recv().await {
            Ok(s) => s,
            Err(e) => {
                warn!("req_receiver.recv error : {}", e);
                continue;
            }
        };

        if data.response.is_closed() {
            warn!("response already closed , skip rendering");
            continue;
        }

        // let aa = [("name", "zhouzhipeng")];
        // aa[0].key();
        let (name, content, use_cache) = match data.template{
            Template::StaticTemplate { name, content } => (name.to_string(),content.to_string(), true),
            Template::DynamicTemplate { name, content } => (name,content, false),
        };

        let args = (content, name, data.args.to_string(), use_cache);

        let r = Python::with_gil(|py| -> PyResult<String> {
            // let syspath: &PyList = py.import("sys")?.getattr("path")?.downcast()?;
            // syspath.insert(0, &path)?;
            // let app: Py<PyAny> =  PyModule::from_code(py, py_app, "", "")?
            //     .getattr("render_tpl_with_str_args")?
            //     .into();
            let r = py_render_fn.call1(py, args)?.to_string();


            Ok(r)
        }).expect("run python error!");

        if data.response.is_closed() {
            warn!("response already closed , skip send back.");
            continue;
        }

        if let Err(e) = data.response.send(r).await {
            error!("py_runner send error : {:?}", e.to_string() );
        }
    }
}


#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    


    #[ignore]
    #[test]
    fn test_re() -> anyhow::Result<()> {
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
    fn test_re2() {
        let tokens = vec!["<%", "%>", "%", "{{", "}}"];
        for t in tokens {
            let escaped_text = regex::escape(t);
            println!("escaped_text :   {}", escaped_text);
        }
    }

    #[ignore]
    #[test]
    fn test_search() {
        use fancy_regex::Regex;
        let re = Regex::new(r#"(?m)([urbURB]?(?:''(?!')|""(?!")|'{6}|"{6}|'(?:[^\\']|\\.)+?'|"(?:[^\\"]|\\.)+?"|'{3}(?:[^\\]|\\.|\n)+?'{3}|"{3}(?:[^\\]|\\.|\n)+?"{3}))|(#.*)|([\[\{\(])|([\]\}\)])|^([ \t]*(?:if|for|while|with|try|def|class)\b)|^([ \t]*(?:elif|else|except|finally)\b)|((?:^|;)[ \t]*end[ \t]*(?=(?:%>[ \t]*)?\r?$|;|#))|(%>[ \t]*(?=\r?$))|(\r?\n)"#).unwrap();
        let hay = r#"<ul>
    % for article in articles:
    <li>
        <a href="/article/{{article.id}}">{{article.title}}</a>
    </li>
    %end

    my name is :{{name}}
</ul>"#;


        let mut start = 0;
        let mut end = 0;
        let mut groups = vec![];
        for mat in re.captures_iter(hay) {
            let mat = mat.unwrap();

            let mut i = 0;
            let mut find = false;
            for mat in mat.iter() {
                let mat = match mat {
                    None => continue,
                    Some(c) => c,
                };
                let g = mat.as_str();
                if i == 0 {
                    start = mat.start();
                    end = mat.end();
                }

                if mat.start() < start {
                    start = mat.start();
                }

                if mat.end() > end {
                    end = mat.end();
                }

                println!("search match  >> \n {}, start = {}, end={}", g, mat.start(), mat.end());

                if i != 0 {
                    groups.push(g);
                } else {
                    find = true;
                }

                i += 1;
            }


            if find {
                break;
            }
        }

        let results = (start, end, groups);

        println!("results >> {:?}", results);
    }


    #[ignore]
    #[test]
    fn test_finditer() {
        use fancy_regex::Regex;
        let re = Regex::new(r"(\w)haha(\w)").unwrap();
        let hay = "aahahabbaahahabb";


        for mat in re.captures_iter(hay) {
            let mat = mat.unwrap();
            let mut groups = vec![];
            let mut i = -1;
            let mut start = 0;
            let mut end = 0;
            for mat in mat.iter() {
                i += 1;
                let mat = mat.unwrap();
                let g = mat.as_str();
                if i == 0 {
                    start = mat.start();
                    end = mat.end();
                    continue;
                }


                if mat.start() < start {
                    start = mat.start();
                }

                if mat.end() > end {
                    end = mat.end();
                }


                if i != 0 {
                    groups.push(g);
                }
            }

            println!("search match  >> \n {:?}, start = {}, end={}", groups, start, end);
        }

        let results = ();

        println!("results >> {:?}", results);
    }
}