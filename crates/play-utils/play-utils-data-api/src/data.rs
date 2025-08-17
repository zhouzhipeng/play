use std::marker::PhantomData;
use anyhow::{anyhow, bail, ensure, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use serde_json::{json, Value};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RawData{
    pub id : i64,
    pub cat: String,
    pub data: String,
    pub created: i64,
    pub updated: i64,
}


impl RawData {
    pub fn to<T:for<'de>  Deserialize<'de> >(&self)-> anyhow::Result<T>{
        serde_json::from_str::<T>(&self.data).context(anyhow!("failed to parse data"))
    }
}

#[derive(Clone, Debug, Default)]
pub struct DataAPI<T>
where T :  Serialize+for<'de>  Deserialize<'de>+Clone{
    pub host: String,
    pub category: String,
    pub auth_key: Option<String>,
    phantom_data: PhantomData<T>
}




impl<T> DataAPI<T>
where T :  Serialize+for<'de>  Deserialize<'de>+Clone
{
    pub fn new(host: &str, category: &str, auth_key: Option<String>)->Self{
        Self{
            host: host.to_string(),
            category: category.to_string(),
            auth_key,
            phantom_data: Default::default(),
        }
    }

    fn get_auth_header(&self)-> String{
        let key = match &self.auth_key{
            None => "".to_string(),
            Some(s) => s.to_string(),
        };
        key

    }

    fn get_client() -> anyhow::Result<Client> {
        let mut builder = Client::builder();
        #[cfg(not(target_arch = "wasm32"))]
        {
            builder = builder.timeout(Duration::from_secs(3));
        }

        Ok(builder
            .build().context("failed to build client")?)
    }

    pub async fn insert(&self, data: &T) -> anyhow::Result<T> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let response = client.post(format!("{}/api/v2/data/{}/insert", self.host, self.category))
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .json(data).send().await?;
        if response.status().is_success(){
            let r: T = response.json().await?;
            Ok(r)
        }else{
            bail!(response.text().await?)
        }

    }

    fn populate_sys_fields(data: &RawData) -> anyhow::Result<T> {
        let mut value = serde_json::from_str::<Value>(&data.data)?;
        value["id"] = json!(&data.id);
        value["created"] = json!(&data.created);
        value["updated"] = json!(&data.updated);
        let t = serde_json::from_value::<T>(value)?;
        Ok(t)
    }

    pub async fn delete( &self, id: i64) -> anyhow::Result<T> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let response = client.delete(format!("{}/data/id/{}", self.host, id))
            .header("X-Browser-Fingerprint", self.get_auth_header()).send().await?;
        if response.status().is_success(){
            let r: Vec<RawData> = response.json().await?;
            Ok(Self::populate_sys_fields(&r[0])?)
        }else{
            bail!(response.text().await?)
        }

    }

    pub async fn delete_by_cat( &self) -> anyhow::Result<u32> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let response = client.get(format!("{}/api/v2/data/{}/delete?delete_all=true&hard_delete=true", self.host, self.category))
            .header("X-Browser-Fingerprint", self.get_auth_header()).send().await?;
        if response.status().is_success(){
            Ok(response.text().await?.parse::<u32>()?)
        }else{
            bail!(response.text().await?)
        }

    }
    pub async fn update_full(&self, id: i64, data: &T) -> anyhow::Result<T> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let response = client.put(format!("{}/data/id/{}", self.host, id))
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .json(data)
            .send().await?;
        if response.status().is_success(){
            let r: Vec<RawData> = response.json().await?;
            Ok(Self::populate_sys_fields(&r[0])?)
        }else{
            bail!(response.text().await?)
        }
    }
    pub async fn update_field(&self, id: i64, field_param: (&str,&str)) -> anyhow::Result<T> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let response = client.patch(format!("{}/data/id/{}", self.host, id))
            .query(&[field_param])
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .send().await?;
        if response.status().is_success(){
            let r: Vec<RawData> = response.json().await?;
            Ok(Self::populate_sys_fields(&r[0])?)
        }else{
            bail!(response.text().await?)
        }
    }
    pub async fn get( &self,id: i64) -> anyhow::Result<T> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let response = client.get(format!("{}/data/cat/{}/id/{}", self.host, self.category, id))
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .send().await?;
        if response.status().is_success(){
            let r: Vec<RawData> = response.json().await?;
            if !r.is_empty(){
                Ok(Self::populate_sys_fields(&r[0])?)
            }else{
                bail!("not found!")
            }


        }else{
            bail!(response.text().await?)
        }
    }
    pub async fn list( &self,limit: u32) -> anyhow::Result<Vec<T>> {
        let raw_data = self.list_raw(limit).await?;
        Ok(raw_data.iter().map(|m|Self::populate_sys_fields(&m).unwrap()).collect())
    }

    pub async fn list_raw(&self,limit: u32) -> anyhow::Result<Vec<RawData>> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let response = client.get(format!("{}/data/cat/{}", self.host, self.category))
            .query(&[("_limit", limit)])
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .send().await?;
        if response.status().is_success(){
            let r: Vec<RawData> = response.json().await?;
            Ok(r)

        }else{
            bail!(response.text().await?)
        }
    }
    pub async fn count(&self) -> anyhow::Result<i64> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let response = client.get(format!("{}/api/v2/data/{}/query?count=true", self.host, self.category))
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .send().await?;
        if response.status().is_success(){
            let r = response.text().await?;
            Ok(r.parse::<i64>()?)

        }else{
            bail!(response.text().await?)
        }
    }
}
