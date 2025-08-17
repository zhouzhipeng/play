use std::str::FromStr;
use anyhow::ensure;
use ethereum_types::Address;
use secp256k1::{PublicKey, SecretKey};
use serde_json::Value;
use sha2::Digest;
use sha3::Keccak256;
use log::info;

pub async fn query_eth_balance(address: &str) ->anyhow::Result<f64>{
    ensure!(address!="");
    let url = format!("https://api.etherscan.io/api?module=account&action=balance&address={}&tag=latest&apikey=Y2CGJ2P89DWWQ7Q2435X5TUTIUYB946V7P", address);
    let resp = reqwest::get(&url).await?.json::<Value>().await?;
    let balance = resp["result"].as_str().unwrap().parse::<f64>()? / 1e18;
    println!("ETH Balance: {}", balance);

    Ok(balance)
}
pub async fn query_usdt_balance(address: &str) ->anyhow::Result<f64>{
    ensure!(address!="");
    let url = format!("https://api.etherscan.io/api?module=account&action=tokenbalance&contractaddress=0xdac17f958d2ee523a2206206994597c13d831ec7&address={}&tag=latest&apikey=Y2CGJ2P89DWWQ7Q2435X5TUTIUYB946V7P", address);
    let resp = reqwest::get(&url).await?.json::<Value>().await?;
    info!("usdt balance : {:?}", resp);
    let balance = resp["result"].as_str().unwrap().parse::<f64>()? / 1e6;
    Ok(balance)
}



pub fn generate_erc20_address(private_key_hex: &str) -> String {
    // 从十六进制字符串解析私钥
    let secret_key = SecretKey::from_str(private_key_hex).unwrap();


    // 生成公钥
    let secp = secp256k1::Secp256k1::new();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    // 序列化公钥
    let public_key = public_key.serialize_uncompressed();

    // 计算 Keccak-256 哈希
    let mut hasher = Keccak256::new();
    hasher.update(&public_key[1..]);
    let hash = hasher.finalize();

    // 取哈希的后 20 字节作为地址
    format!("{:?}", Address::from_slice(&hash[12..]))
}