pub mod data;
#[cfg(test)]
mod tests {
    use std::thread;
    use serde::{Deserialize, Serialize};
    use crate::data::DataAPI;
    use super::*;


    #[derive(Serialize, Deserialize, Debug, Clone, Default)]
    struct Demo{
        name: String,
    }

    #[tokio::test]
    async fn test_insert()->anyhow::Result<()> {
        let api = DataAPI::<Demo>::new("http://127.0.0.1:3000", "demo",None);
        let r = api.insert(&Demo{ name: "demo name222".to_string(), ..Default::default() } ).await;
        println!("{:?}", r);

        Ok(())
    }

    #[tokio::test]
    async fn test_count()->anyhow::Result<()> {
        let api = DataAPI::<Demo>::new("http://127.0.0.1:3000", "demo",None);
        let r = api.count().await;
        println!("{:?}", r);

        Ok(())
    }
    #[tokio::test]
    async fn test_delete()->anyhow::Result<()> {
        let api = DataAPI::<Demo>::new("http:127.0.0.1:9000", "demo",None);

        let r = api.delete(9759).await?;
        println!("{:?}", r);

        Ok(())
    }
    #[tokio::test]
    async fn test_delete_by_cat()->anyhow::Result<()> {
        let api = DataAPI::<Demo>::new("http:127.0.0.1:3000", "demo",None);

        let r = api.delete_by_cat().await?;
        println!("{:?}", r);

        Ok(())
    }
    #[tokio::test]
    async fn test_update_full()->anyhow::Result<()> {
        let api = DataAPI::<Demo>::new("http:127.0.0.1:9000", "demo",None);

        let r = api.update_full(9760, &Demo{ name: "501 name".to_string(), ..Default::default() }).await?;
        println!("{:?}", r);

        Ok(())
    }
    #[tokio::test]
    async fn test_update_field()->anyhow::Result<()> {
        let api = DataAPI::<Demo>::new("http:127.0.0.1:9000", "demo",None);

        let r = api.update_field(9760, ("name", "502 name")).await?;
        println!("{:?}", r);

        Ok(())
    }
    #[tokio::test]
    async fn test_query_by_id()->anyhow::Result<()> {
        let api = DataAPI::<Demo>::new("http:127.0.0.1:9000", "demo",None);

        let r = api.get(500).await?;
        println!("{:?}", r);

        Ok(())
    }
    #[tokio::test]
    async fn test_query_by_category()->anyhow::Result<()> {
        // let api = DataAPI::<Demo>::new("http:127.0.0.1:9000", "demo",None);

        // let r = api.list(100).await?;
        let r =reqwest::get("http://127.0.0.1:9000/data/cat/demo").await?.text().await?;
        println!("{:?}", r);

        Ok(())
    }
}
