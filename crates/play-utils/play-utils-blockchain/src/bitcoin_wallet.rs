use bitcoin::key::Secp256k1;
use bitcoin::{Address, Network, PrivateKey, PublicKey};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use anyhow::{bail, ensure};
use serde_json::Value;

pub fn gen_btc_address(private_key: &str) ->String{
    let secp = Secp256k1::new();

    // 从指定的私钥字符串生成 SecretKey
    let private_key = PrivateKey::from_str(&private_key_to_wif(private_key)).unwrap();

    // 生成公钥
    let public_key = PublicKey::from_private_key(&secp, &private_key);


    // 创建比特币地址
    let address = Address::p2pkh(&public_key, Network::Bitcoin);

    format!("{address}")
}



pub fn private_key_to_wif(private_key_hex: &str) -> String {
    // 1. 将私钥从16进制转换为字节
    let mut private_key_bytes = hex::decode(private_key_hex).expect("Invalid Hex");

    // 2. 添加 WIF 前缀 (0x80 for mainnet, 0xef for testnet)
    private_key_bytes.insert(0, 0x80);

    // 3. 对扩展后的私钥进行两次 SHA-256 哈希
    let sha256_1 = Sha256::digest(&private_key_bytes);
    let sha256_2 = Sha256::digest(&sha256_1);

    // 4. 取前4个字节作为校验和
    let checksum = &sha256_2[0..4];

    // 5. 将校验和附加到私钥后面
    private_key_bytes.extend_from_slice(checksum);

    // 6. Base58 编码
    bs58::encode(private_key_bytes).into_string()
}


pub async fn query_btc_balance(address: &str) ->anyhow::Result<f64>{
    ensure!(address!="");

    let url = format!("https://blockchain.info/balance?active={}", address);

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let body = response.text().await?;
    let json: Value = serde_json::from_str(&body)?;

    if let Some(balance) = json[address]["final_balance"].as_u64() {

        Ok((balance as f64) / 100_000_000.0)
    } else {
        bail!("无法获取余额信息");
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_private_key_to_wif() {
        let pk = private_key_to_wif("d7697570462f7562b83e81258de0f1e41832e98072e44c36ec8efec46786e24e");;

        println!("{}", pk);
    }
}