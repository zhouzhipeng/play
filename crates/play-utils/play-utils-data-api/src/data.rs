use std::marker::PhantomData;
use anyhow::{anyhow, bail, ensure, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use serde_json::{json, Value};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RawData{
    pub id : u32,
    pub cat: String,
    pub data: String,
    pub is_deleted: bool,
    pub created: i64,
    pub updated: i64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct AffectedRows {
    pub affected_rows: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CountResult {
    pub rows: u32,
}


impl RawData {
    pub fn to<T:for<'de>  Deserialize<'de> >(&self)-> anyhow::Result<T>{
        serde_json::from_str::<T>(&self.data).context(anyhow!("failed to parse data"))
    }
    
    /// Convert to flattened object including system fields (same as v4 API response)
    pub fn to_flat<T: for<'de> Deserialize<'de>>(&self) -> anyhow::Result<T> {
        let mut value = serde_json::from_str::<Value>(&self.data)?;
        value["id"] = json!(self.id);
        value["cat"] = json!(&self.cat);
        value["is_deleted"] = json!(self.is_deleted);
        value["created"] = json!(self.created);
        value["updated"] = json!(self.updated);
        let t = serde_json::from_value::<T>(value)?;
        Ok(t)
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

    /// Insert data with option to ensure uniqueness
    pub async fn insert(&self, data: &T) -> anyhow::Result<T> {
        self.insert_with_unique(data, false).await
    }
    
    /// Insert data with unique option. If unique=true, only one record can exist for this category
    pub async fn insert_with_unique(&self, data: &T, unique: bool) -> anyhow::Result<T> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let mut url = format!("{}/api/v4/data/{}/insert", self.host, self.category);
        if unique {
            url = format!("{}?unique=true", url);
        }

        let response = client.post(&url)
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .json(data).send().await?;
        if response.status().is_success(){
            let r: T = response.json().await?;
            Ok(r)
        }else{
            bail!(response.text().await?)
        }
    }


    /// Delete a single entry by ID (soft delete by default)
    pub async fn delete(&self, id: u32) -> anyhow::Result<AffectedRows> {
        self.delete_with_options(Some(id), false, false).await
    }
    
    /// Delete with full options: specific ID, delete_all flag, and hard_delete flag
    pub async fn delete_with_options(&self, id: Option<u32>, delete_all: bool, hard_delete: bool) -> anyhow::Result<AffectedRows> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let mut url = format!("{}/api/v4/data/{}/delete?", self.host, self.category);
        let mut params = vec![];
        
        if let Some(id) = id {
            params.push(format!("id={}", id));
        }
        if delete_all {
            params.push("delete_all=true".to_string());
        }
        if hard_delete {
            params.push("hard_delete=true".to_string());
        }
        
        url.push_str(&params.join("&"));

        let response = client.post(&url)
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .send().await?;
        if response.status().is_success(){
            let r: AffectedRows = response.json().await?;
            Ok(r)
        }else{
            bail!(response.text().await?)
        }
    }

    /// Delete all entries in the category (hard delete)
    pub async fn delete_by_cat(&self) -> anyhow::Result<AffectedRows> {
        self.delete_with_options(None, true, true).await
    }
    /// Update data by replacing the entire data object
    pub async fn update_full(&self, id: u32, data: &T) -> anyhow::Result<AffectedRows> {
        self.update_with_options(id, data, true).await
    }
    
    /// Update data with JSON patch (partial update)
    pub async fn update_patch(&self, id: u32, data: &T) -> anyhow::Result<AffectedRows> {
        self.update_with_options(id, data, false).await
    }
    
    /// Update with override_data option
    pub async fn update_with_options(&self, id: u32, data: &T, override_data: bool) -> anyhow::Result<AffectedRows> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let mut url = format!("{}/api/v4/data/{}/update?id={}", self.host, self.category, id);
        if override_data {
            url = format!("{}&override_data=true", url);
        }

        let response = client.post(&url)
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .json(data)
            .send().await?;
        if response.status().is_success(){
            let r: AffectedRows = response.json().await?;
            Ok(r)
        }else{
            bail!(response.text().await?)
        }
    }
    /// Update a single field using JSON patch
    pub async fn update_field(&self, id: u32, field_name: &str, field_value: &Value) -> anyhow::Result<AffectedRows> {
        let mut data = serde_json::Map::new();
        data.insert(field_name.to_string(), field_value.clone());
        let update_data = Value::Object(data);
        
        self.update_patch(id, &serde_json::from_value::<T>(update_data)?).await
    }
    /// Get a single entry by ID
    pub async fn get(&self, id: u32) -> anyhow::Result<T> {
        self.get_with_options(Some(id), None, false).await
    }
    
    /// Get single entry when there's exactly one record in the category
    pub async fn get_unique(&self) -> anyhow::Result<T> {
        self.get_with_options(None, None, false).await
    }
    
    /// Get with full options: ID, field selection, and slim format
    pub async fn get_with_options(&self, id: Option<u32>, select: Option<&str>, slim: bool) -> anyhow::Result<T> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let mut url = format!("{}/api/v4/data/{}/get?", self.host, self.category);
        let mut params = vec![];
        
        if let Some(id) = id {
            params.push(format!("id={}", id));
        }
        if let Some(select) = select {
            params.push(format!("select={}", select));
        }
        if slim {
            params.push("slim=true".to_string());
        }
        
        url.push_str(&params.join("&"));

        let response = client.get(&url)
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .send().await?;
        if response.status().is_success(){
            let r: T = response.json().await?;
            Ok(r)
        }else{
            bail!(response.text().await?)
        }
    }
    /// Query entries with simple limit
    pub async fn list(&self, limit: u32) -> anyhow::Result<Vec<T>> {
        self.query_with_options(None, Some(&format!("0,{}", limit)), None, None, false, false).await
    }
    
    /// Query with full options
    pub async fn query_with_options(
        &self,
        select: Option<&str>,
        limit: Option<&str>,
        where_clause: Option<&str>,
        order_by: Option<&str>,
        slim: bool,
        include_deleted: bool,
    ) -> anyhow::Result<Vec<T>> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let mut url = format!("{}/api/v4/data/{}/query?", self.host, self.category);
        let mut params = vec![];
        
        if let Some(select) = select {
            params.push(format!("select={}", select));
        }
        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(where_clause) = where_clause {
            params.push(format!("where={}", urlencoding::encode(where_clause)));
        }
        if let Some(order_by) = order_by {
            params.push(format!("order_by={}", urlencoding::encode(order_by)));
        }
        if slim {
            params.push("slim=true".to_string());
        }
        if include_deleted {
            params.push("include_deleted=true".to_string());
        }
        
        url.push_str(&params.join("&"));

        let response = client.get(&url)
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .send().await?;
        if response.status().is_success(){
            let r: Vec<T> = response.json().await?;
            Ok(r)
        }else{
            bail!(response.text().await?)
        }
    }
    /// Count entries in the category
    pub async fn count(&self) -> anyhow::Result<u32> {
        self.count_with_options(None, false).await
    }
    
    /// Count with where clause and include_deleted option
    pub async fn count_with_options(&self, where_clause: Option<&str>, include_deleted: bool) -> anyhow::Result<u32> {
        ensure!(!self.category.is_empty());
        let client = Self::get_client()?;

        let mut url = format!("{}/api/v4/data/{}/count?", self.host, self.category);
        let mut params = vec![];
        
        if let Some(where_clause) = where_clause {
            params.push(format!("where={}", urlencoding::encode(where_clause)));
        }
        if include_deleted {
            params.push("include_deleted=true".to_string());
        }
        
        url.push_str(&params.join("&"));

        let response = client.get(&url)
            .header("X-Browser-Fingerprint", self.get_auth_header())
            .send().await?;
        if response.status().is_success(){
            let r: CountResult = response.json().await?;
            Ok(r.rows)
        }else{
            bail!(response.text().await?)
        }
    }
}
