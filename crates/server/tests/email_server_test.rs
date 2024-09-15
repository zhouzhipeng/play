use play_mail_server::models::message::Message;
use play::handle_email_message;
use play::init_app_state;
use play::tables::email_inbox::EmailInbox;

#[ignore]
#[tokio::test]
async fn test_save_message() -> anyhow::Result<()> {
    let app_state = init_app_state(&play::config::init_config(true), true).await;

    handle_email_message(&app_state,&Message{
        id: Some(1),
        sender: "aa@qq.com".to_string(),
        recipients: vec!["bb@cc.com".to_string(),"111@cc.com".to_string()],
        subject: "test111".to_string(),
        created_at: Some("10:11".to_string()),
        attachments: vec![],
        source: vec![],
        formats: vec![],
        html: Some("test html content".to_string()),
        plain: Some("test html content".to_string()),
    }).await;

    let items = EmailInbox::query_all(&app_state.db).await?;
    println!("items >> {:?}", items);

    assert_eq!(items.len(),1);


    Ok(())
}
