use std::time::Duration;
use http_body::Body;
use play::mock_server;
use play::tables::general_data::GeneralData;

#[tokio::test]
async fn integration_test() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert json
    let resp = server.post("/data/foo/insert").text(r#"
    {"name":"zzp", "title": "t1", "url":"/xx", "content": "123"}
    "#).await;
    resp.assert_status_ok();

    //insert text
    let resp = server.post("/data/bar/insert").text(r#"
    this is a text
    "#).await;
    resp.assert_status_ok();

    //get
    let resp = server.get("/data/get/1").await;
    resp.assert_status_ok();
    let data = resp.json::<Option<GeneralData>>();
    println!("get data : {:?}", data);
    assert!(data.is_some());

    //query by field
    let resp = server.get("/data/foo/query")
        .add_query_param("name", "zzp")
        .await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("query data : {:?}", data);
    assert_eq!(data.len(),1);

    let resp = server.get("/data/get/2").await;
    resp.assert_status_ok();
    let data = resp.json::<Option<GeneralData>>();
    assert_eq!(data.unwrap().data, "this is a text");


    // //query list
    let resp = server.get("/data/foo/list").await;
    resp.assert_status_ok();
    let data = resp.text();
    println!("list data : {}", data);
    let resp = server.get("/data/bar/list").await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("list data : {:?}", data);


    //update field
    // tokio::time::sleep(Duration::from_secs(3)).await;
    let resp = server.put("/data/update-field/1")
        .add_query_param("name", "zzp2")
        .await;
    println!("resp >> {:?}", resp);
    resp.assert_status_ok();
    let resp = server.get("/data/foo/list").await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("list data : {:?}", data);


    //update data
    let resp = server.put("/data/update-data/1")
        .add_query_param("data", "abc")
        .await;
    println!("resp >> {:?}", resp);
    resp.assert_status_ok();
    let resp = server.get("/data/foo/list").await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("list data : {:?}", data);


    //delete
    let resp = server.delete("/data/delete/1")
        .await;
    println!("resp >> {:?}", resp);
    resp.assert_status_ok();
    let resp = server.get("/data/foo/list").await;
    resp.assert_status_ok();
    let data = resp.json::<Vec<GeneralData>>();
    println!("list data : {:?}", data);


    Ok(())
}
