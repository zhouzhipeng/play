use play::mock_server;
use play::tables::general_data::GeneralData;

#[tokio::test]
async fn integration_test() -> anyhow::Result<()> {
    let server = mock_server!();

    //insert
    let resp = server.post("/general-data/1/insert").text(r#"
    {"name":"zzp"}
    "#).await;
    resp.assert_status_ok();
    println!("resp : {}", resp.text());

    //query list
    let data = server.get("/general-data/1/list").await.json::<Vec<GeneralData>>();
    println!("list data : {:?}", data);
    assert_eq!(data.len(),1);
    assert_eq!(data[0].meta_id, 1);

    //query by name
    let data = server.get("/general-data/1/query")
        .add_query_param("name", "zzp")
        .await.json::<Vec<GeneralData>>();
    println!("query data : {:?}", data);


    Ok(())
}
