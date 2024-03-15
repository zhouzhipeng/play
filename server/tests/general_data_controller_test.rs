use play::mock_server;
use play::tables::general_data::GeneralData;

#[tokio::test]
async fn integration_test() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert
    let resp = server.post("/data/1/insert").text(r#"
    {"name":"zzp"}
    "#).await;
    println!("resp : {}", resp.text());
    resp.assert_status_ok();


    let resp = server.post("/data/1/insert").text(r#"
    {"name":"zzp"}
    "#).await;
    resp.assert_status_ok();
    println!("resp : {}", resp.text());

    // //query list
    let resp = server.get("/data/1/list").await;
    resp.assert_status_ok();
    let data = resp.text();
    println!("list data : {:?}", data);
    // assert_eq!(data.len(),1);
    // assert_eq!(data[0].meta_id, 1);

    //query by name
    let data = server.get("/data/1/query")
        .add_query_param("name", "zzp")
        .await.text();
    println!("query data : {:?}", data);

    //update
    let resp = server.put("/data/update/1")
        .add_query_param("name", "zzp2")
        .await;
    println!("resp >> {:?}", resp);
    resp.assert_status_ok();
    println!("resp : {}", resp.text());

    //query by name
    let data = server.get("/data/1/query")
        .add_query_param("name", "zzp")
        .await.text();
    println!("query data : {:?}", data);

    //delete
    let resp = server.delete("/data/delete/1")
        .await;
    println!("resp >> {:?}", resp);
    resp.assert_status_ok();
    println!("resp : {}", resp.text());

    //query by name
    let data = server.get("/data/1/query")
        .add_query_param("id", "2")
        .await.text();
    println!("query data : {:?}", data);
    //query by id
    let data = server.get("/data/get/2")
        .await.text();
    println!("query data : {:?}", data);

    Ok(())
}
