use criterion::{black_box, criterion_group, criterion_main, Criterion};
use reqwest::blocking::Client;
use tracing::error;


pub fn template_render_test(c: &mut Criterion) {
    c.bench_function("template render test", |b| b.iter(|| {
        //test code
        let client = Client::builder().build().unwrap();
        let res = client.get("http://localhost:3000/page/article/list/v2").send().unwrap();
        // assert_eq!(code.is_success(), true);
        if !res.status().is_success(){
            error!("error >> {:?}",res.text())
        }
    }));
}

criterion_group!(benches, template_render_test);
criterion_main!(benches);
