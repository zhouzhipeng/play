use std::time::Duration;
use http_body::Body;
use play::mock_server;
use play::tables::general_data::GeneralData;

#[tokio::test]
async fn integration_test() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert json
    let resp = server.post("/data/cat-foo").text(r#"
    {"name":"zzp", "title": "t1", "url":"/xx", "content": "123"}
    "#).await;
    resp.assert_status_ok();

    //insert text
    let resp = server.post("/data/cat-bar").text(r#"
    this is a text
    "#).await;
    resp.assert_status_ok();

    //get
    let resp = server.get("/data/id-1").await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("get data : {:?}", data);

    //query by field
    let resp = server.get("/data/cat-foo")
        .add_query_param("name", "zzp")
        .await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("query data : {:?}", data);
    assert_eq!(data.len(),1);

    let resp = server.get("/data/id-2").await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    assert_eq!(data[0].data, "this is a text");


    // //query list
    let resp = server.get("/data/cat-foo").await;
    resp.assert_status_ok();
    let data = resp.text();
    println!("list data : {}", data);
    let resp = server.get("/data/cat-bar").await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("list data : {:?}", data);


    //update field
    // tokio::time::sleep(Duration::from_secs(3)).await;
    let resp = server.put("/data/id-1")
        .add_query_param("name", "zzp2")
        .await;
    println!("resp >> {:?}", resp);
    resp.assert_status_ok();
    let resp = server.get("/data/cat-foo").await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("list data : {:?}", data);


    //update data
    let resp = server.put("/data/id-1")
        .add_query_param("data", "abc")
        .await;
    println!("resp >> {:?}", resp);
    resp.assert_status_ok();
    let resp = server.get("/data/cat-foo").await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("list data : {:?}", data);


    //delete
    let resp = server.delete("/data/id-1")
        .await;
    println!("resp >> {:?}", resp);
    resp.assert_status_ok();
    let resp = server.get("/data/cat-foo").await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("list data : {:?}", data);


    Ok(())
}

#[tokio::test]
async fn global_cat_test() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert
    let resp = server.patch("/data/cat-foo").text(r#"
    123
    "#).await;
    println!("insert resp : {:?}", resp.text());
    resp.assert_status_ok();

    let resp = server.post("/data/cat-foo").text(r#"
    456
    "#).await;
    println!("insert resp : {:?}", resp.text());
    resp.assert_status_ok();

    //query
    let resp = server.get("/data/cat-foo").await;
    println!("get data : {}", resp.text());
    resp.assert_status_ok();

    let resp = server.get("/data/id-1").await;
    println!("get data : {}", resp.text());
    resp.assert_status_ok();


    Ok(())
}

#[tokio::test]
async fn patch_update_test() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert
    let resp = server.post("/data/cat-foo").text(r#"
    {"name":"zzp", "age":18}
    "#).await;
    println!("insert resp : {:?}", resp.text());
    resp.assert_status_ok();

    //update patch
    let resp = server.patch("/data/id-1")
        .add_query_param("name","zzp2")
        .await;
    println!("insert resp : {:?}", resp.text());
    resp.assert_status_ok();

    //query
    let resp = server.get("/data/cat-foo").await;
    println!("get data : {}", resp.text());
    resp.assert_status_ok();
    //query
    let resp = server.get("/data/id-1").await;
    println!("get data : {}", resp.text());
    resp.assert_status_ok();


    Ok(())
}
