use anyhow::anyhow;
use std::collections::BTreeMap;
use std::path::Path;
use std::{env, fs};

use serde::{Deserialize, Serialize};
use serde_json::json;

use tracing::{error, info};

use crate::render_template_new;
use play_shared::constants::DATA_DIR;
use play_shared::file_path;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_log_level")]
    pub log_level: String,
    pub server_port: u32,
    #[serde(default)]
    pub use_test_pool: bool,
    #[serde(default)]
    pub redis_uri: Vec<String>,
    #[serde(default)]
    pub redis_url: Option<String>,
    pub database: Database,
    #[serde(default)]
    pub upgrade_url: String,
    #[serde(default)]
    pub https_cert: HttpsCert,
    #[serde(default)]
    pub shortlinks: Vec<ShortLink>,
    #[serde(default)]
    pub auth_config: AuthConfig,
    #[serde(default)]
    pub domain_proxy: Vec<DomainProxy>,
    #[serde(default)]
    pub misc_config: MiscConfig,
    #[serde(default)]
    pub cache_config: CacheConfig,
    #[serde(default)]
    pub plugin_config: Vec<PluginConfig>,
    #[serde(default)]
    pub frp_server: FrpServerConfig,
    #[serde(default)]
    pub ikev2_server: Ikev2ServerConfig,

    #[cfg(feature = "play-integration-xiaozhi")]
    #[serde(default)]
    pub mcp_config: play_integration_xiaozhi::McpConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct CacheConfig {
    #[serde(default)]
    pub cf_token: String,
    #[serde(default)]
    pub cf_purge_cache_url: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct PluginConfig {
    #[serde(default)]
    pub proxy_domain: String,
    #[serde(default)]
    pub url_prefix: String,
    #[serde(default)]
    pub file_path: String,
    #[serde(default)]
    pub name: String,
    /// if true, will load&render config.toml content and pass to plugin execution.
    #[serde(default)]
    pub render_config: bool,
    #[serde(default)]
    pub is_server: bool,
    #[serde(default)]
    pub disable: bool,
    #[serde(default)]
    pub create_process: bool,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DomainProxy {
    #[serde(default)]
    pub proxy_domain: String,
    #[serde(flatten)]
    pub proxy_target: ProxyTarget,
    /// 是否使用HTTPS协议（默认根据端口自动判断：443为https，其他为http）
    #[serde(default)]
    pub use_https: Option<bool>,
    /// 是否忽略HTTPS证书警告（默认false，仅在开发环境使用）
    #[serde(default)]
    pub ignore_cert: bool,
    #[serde(flatten)]
    pub websocket_config: WebSocketConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OriginStrategy {
    /// 保持原始Origin头部不变
    Keep,
    /// 移除Origin头部
    Remove,
    /// 设置为代理域名
    Host,
    /// 设置为后端服务器地址
    Backend,
    /// 使用自定义Origin值
    Custom,
}

impl Default for OriginStrategy {
    fn default() -> Self {
        OriginStrategy::Keep
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WebSocketConfig {
    /// WebSocket Origin处理策略
    #[serde(default)]
    pub origin_strategy: OriginStrategy,
    /// 自定义Origin值（当origin_strategy为"custom"时使用）
    #[serde(default)]
    pub custom_origin: Option<String>,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            origin_strategy: OriginStrategy::Keep,
            custom_origin: None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ProxyTarget {
    #[serde(rename = "folder")]
    Folder {
        #[serde(default)]
        folder_path: String,
    },
    #[serde(rename = "upstream")]
    Upstream {
        #[serde(default = "default_proxy_ip")]
        ip: String,
        #[serde(default = "default_proxy_port")]
        port: u16,
    },
}

impl Default for DomainProxy {
    fn default() -> Self {
        Self {
            proxy_domain: String::default(),
            proxy_target: ProxyTarget::Folder {
                folder_path: String::default(),
            },
            use_https: None,
            ignore_cert: false,
            websocket_config: WebSocketConfig::default(),
        }
    }
}

fn default_proxy_ip() -> String {
    "127.0.0.1".to_string()
}
fn default_proxy_port() -> u16 {
    80
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct ShortLink {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub download: bool,
    #[serde(default)]
    pub auth: bool,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct AuthConfig {
    pub enabled: bool,
    pub fingerprints: Vec<String>,
    pub whitelist: Vec<String>,
    pub passcode: String,
}
#[derive(Deserialize, Debug, Clone, Default)]
pub struct MiscConfig {
    #[serde(default)]
    pub mail_notify_url: String,
    #[serde(default)]
    pub github_token: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
pub enum FrpServiceType {
    #[default]
    Tcp,
    Udp,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, Default)]
pub enum FrpTransportType {
    #[default]
    #[serde(rename = "tcp")]
    Tcp,
    #[serde(rename = "tls")]
    Tls,
    #[serde(rename = "noise")]
    Noise,
    #[serde(rename = "websocket")]
    Websocket,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FrpServerServiceConfig {
    #[serde(rename = "type", default)]
    pub service_type: FrpServiceType,
    #[serde(default)]
    pub bind_addr: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub nodelay: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FrpTlsConfig {
    #[serde(default)]
    pub hostname: Option<String>,
    #[serde(default)]
    pub trusted_root: Option<String>,
    #[serde(default)]
    pub pkcs12: Option<String>,
    #[serde(default)]
    pub pkcs12_password: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FrpNoiseConfig {
    #[serde(default = "default_frp_noise_pattern")]
    pub pattern: String,
    #[serde(default)]
    pub local_private_key: Option<String>,
    #[serde(default)]
    pub remote_public_key: Option<String>,
}

impl Default for FrpNoiseConfig {
    fn default() -> Self {
        Self {
            pattern: default_frp_noise_pattern(),
            local_private_key: None,
            remote_public_key: None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FrpWebsocketConfig {
    #[serde(default)]
    pub tls: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FrpTcpConfig {
    #[serde(default = "default_frp_nodelay")]
    pub nodelay: bool,
    #[serde(default = "default_frp_keepalive_secs")]
    pub keepalive_secs: u64,
    #[serde(default = "default_frp_keepalive_interval")]
    pub keepalive_interval: u64,
    #[serde(default)]
    pub proxy: Option<String>,
}

impl Default for FrpTcpConfig {
    fn default() -> Self {
        Self {
            nodelay: default_frp_nodelay(),
            keepalive_secs: default_frp_keepalive_secs(),
            keepalive_interval: default_frp_keepalive_interval(),
            proxy: None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct FrpTransportConfig {
    #[serde(rename = "type", default)]
    pub transport_type: FrpTransportType,
    #[serde(default)]
    pub tcp: FrpTcpConfig,
    #[serde(default)]
    pub tls: Option<FrpTlsConfig>,
    #[serde(default)]
    pub noise: Option<FrpNoiseConfig>,
    #[serde(default)]
    pub websocket: Option<FrpWebsocketConfig>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct FrpServerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_frp_server_bind_addr")]
    pub bind_addr: String,
    #[serde(default)]
    pub default_token: Option<String>,
    #[serde(default)]
    pub services: BTreeMap<String, FrpServerServiceConfig>,
    #[serde(default)]
    pub transport: FrpTransportConfig,
    #[serde(default = "default_frp_heartbeat_interval")]
    pub heartbeat_interval: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Ikev2ServerConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ikev2_auto_install_dependencies")]
    pub auto_install_dependencies: bool,
    #[serde(default = "default_ikev2_daemon_bin")]
    pub daemon_bin: String,
    #[serde(default = "default_ikev2_swanctl_bin")]
    pub swanctl_bin: String,
    #[serde(default = "default_ikev2_listen_addr")]
    pub listen_addr: String,
    #[serde(default = "default_ikev2_port")]
    pub port: u16,
    #[serde(default = "default_ikev2_port_nat_t")]
    pub port_nat_t: u16,
    #[serde(default = "default_ikev2_connection_name")]
    pub connection_name: String,
    #[serde(default)]
    pub local_id: String,
    #[serde(default)]
    pub server_cert: String,
    #[serde(default)]
    pub server_key: String,
    #[serde(default)]
    pub ca_cert: Option<String>,
    #[serde(default = "default_ikev2_pool")]
    pub pool: String,
    #[serde(default = "default_ikev2_local_ts")]
    pub local_ts: Vec<String>,
    #[serde(default)]
    pub dns_servers: Vec<String>,
    #[serde(default)]
    pub eap_users: BTreeMap<String, String>,
    #[serde(default = "default_ikev2_fragmentation")]
    pub fragmentation: bool,
    #[serde(default = "default_ikev2_mobike")]
    pub mobike: bool,
    #[serde(default = "default_ikev2_dpd_delay_secs")]
    pub dpd_delay_secs: u64,
    #[serde(default)]
    pub proposals: Option<String>,
    #[serde(default)]
    pub esp_proposals: Option<String>,
    #[serde(default = "default_ikev2_log_level")]
    pub log_level: u8,
    #[serde(default = "default_ikev2_startup_timeout_secs")]
    pub startup_timeout_secs: u64,
}

impl Default for FrpServerConfig {
    fn default() -> Self {
        let mut services = BTreeMap::new();
        services.insert(
            "demo_http".to_string(),
            FrpServerServiceConfig {
                service_type: FrpServiceType::Tcp,
                bind_addr: "0.0.0.0:8081".to_string(),
                token: None,
                nodelay: None,
            },
        );

        Self {
            enabled: false,
            bind_addr: default_frp_server_bind_addr(),
            default_token: Some("change_this_token".to_string()),
            services,
            transport: FrpTransportConfig::default(),
            heartbeat_interval: default_frp_heartbeat_interval(),
        }
    }
}

impl Default for Ikev2ServerConfig {
    fn default() -> Self {
        let mut eap_users = BTreeMap::new();
        eap_users.insert("demo".to_string(), "change_this_password".to_string());

        Self {
            enabled: false,
            auto_install_dependencies: default_ikev2_auto_install_dependencies(),
            daemon_bin: default_ikev2_daemon_bin(),
            swanctl_bin: default_ikev2_swanctl_bin(),
            listen_addr: default_ikev2_listen_addr(),
            port: default_ikev2_port(),
            port_nat_t: default_ikev2_port_nat_t(),
            connection_name: default_ikev2_connection_name(),
            local_id: "vpn.example.com".to_string(),
            server_cert: "certs/ikev2/server-cert.pem".to_string(),
            server_key: "certs/ikev2/server-key.pem".to_string(),
            ca_cert: Some("certs/ikev2/ca-cert.pem".to_string()),
            pool: default_ikev2_pool(),
            local_ts: default_ikev2_local_ts(),
            dns_servers: vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()],
            eap_users,
            fragmentation: default_ikev2_fragmentation(),
            mobike: default_ikev2_mobike(),
            dpd_delay_secs: default_ikev2_dpd_delay_secs(),
            proposals: None,
            esp_proposals: None,
            log_level: default_ikev2_log_level(),
            startup_timeout_secs: default_ikev2_startup_timeout_secs(),
        }
    }
}

fn default_log_level() -> String {
    "INFO".to_string()
}

fn default_frp_server_bind_addr() -> String {
    "0.0.0.0:2333".to_string()
}

fn default_frp_heartbeat_interval() -> u64 {
    30
}

fn default_frp_noise_pattern() -> String {
    "Noise_NK_25519_ChaChaPoly_BLAKE2s".to_string()
}

fn default_frp_nodelay() -> bool {
    true
}

fn default_frp_keepalive_secs() -> u64 {
    20
}

fn default_frp_keepalive_interval() -> u64 {
    8
}

fn default_ikev2_daemon_bin() -> String {
    "charon-systemd".to_string()
}

fn default_ikev2_auto_install_dependencies() -> bool {
    true
}

fn default_ikev2_swanctl_bin() -> String {
    "swanctl".to_string()
}

fn default_ikev2_listen_addr() -> String {
    "0.0.0.0".to_string()
}

fn default_ikev2_port() -> u16 {
    500
}

fn default_ikev2_port_nat_t() -> u16 {
    4500
}

fn default_ikev2_connection_name() -> String {
    "play-ikev2".to_string()
}

fn default_ikev2_pool() -> String {
    "10.10.10.0/24".to_string()
}

fn default_ikev2_local_ts() -> Vec<String> {
    vec!["0.0.0.0/0".to_string(), "::/0".to_string()]
}

fn default_ikev2_fragmentation() -> bool {
    true
}

fn default_ikev2_mobike() -> bool {
    true
}

fn default_ikev2_dpd_delay_secs() -> u64 {
    30
}

fn default_ikev2_log_level() -> u8 {
    2
}

fn default_ikev2_startup_timeout_secs() -> u64 {
    15
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct HttpsCert {
    pub https_port: u16,
    #[serde(default)]
    pub auto_redirect: bool,
    /// first domain is main domain ,other domain will serve folder under $files/$domain_name
    pub domains: Vec<String>,
    pub emails: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Database {
    pub url: String,
}

const CONFIG: &str = include_str!(file_path!("/config.toml"));

pub async fn read_config_file(render_lua: bool) -> anyhow::Result<String> {
    let file_path = format!("config.toml");
    let final_path = Path::new(env::var(DATA_DIR)?.as_str()).join(file_path.as_str());

    // info!("config path : {:?}", final_path);

    // If config file doesn't exist, create it with default content
    if !final_path.exists() {
        let default_config = r#"server_port = 3000
log_level = "DEBUG"

[database]
url=":memory:"

[frp_server]
enabled = false
bind_addr = "0.0.0.0:2333"
default_token = "change_this_token"
heartbeat_interval = 30

[frp_server.services.demo_http]
bind_addr = "0.0.0.0:8081"

[ikev2_server]
enabled = false
auto_install_dependencies = true
listen_addr = "0.0.0.0"
port = 500
port_nat_t = 4500
local_id = "vpn.example.com"
server_cert = "certs/ikev2/server-cert.pem"
server_key = "certs/ikev2/server-key.pem"
ca_cert = "certs/ikev2/ca-cert.pem"
pool = "10.10.10.0/24"
dns_servers = ["1.1.1.1", "8.8.8.8"]

[ikev2_server.eap_users]
demo = "change_this_password"

"#;
        fs::write(&final_path, default_config)?;
        info!("Created default config file at {:?}", final_path);
    }

    let mut content = fs::read_to_string(&final_path)?;

    if render_lua {
        // run lua template
        content = render_template_new(&content, json!({})).await?
    }

    Ok(content)
}
pub fn save_config_file(content: &str) -> anyhow::Result<()> {
    let file_path = format!("config.toml");
    let final_path = Path::new(env::var(DATA_DIR)?.as_str()).join(file_path.as_str());

    // info!("config path : {:?}", final_path);

    fs::write(&final_path, content)?;
    Ok(())
}

pub fn get_config_path() -> anyhow::Result<String> {
    let file_path = format!("config.toml");
    let final_path = Path::new(env::var(DATA_DIR)?.as_str()).join(file_path.as_str());

    // info!("config path : {:?}", final_path);
    Ok(final_path.to_str().unwrap().to_string())
}

pub async fn init_config(render_lua: bool) -> anyhow::Result<Config> {
    let config_content = read_config_file(render_lua).await?;

    let config: Config = toml::from_str(&config_content)?;
    //println!("using config file  content >>  {:?}",  config);
    Ok(config)
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.

    use super::*;
    use play_shared::constants::HOST;
    use play_shared::get_workspace_root;

    #[tokio::test]
    async fn test_read_config_file() -> anyhow::Result<()> {
        env::set_var(
            DATA_DIR,
            format!("{}{}", get_workspace_root(), "/server/output_dir"),
        );
        env::set_var("HOST", "http://localhost:3000");
        let content = read_config_file(true).await?;
        println!("{}", content);
        Ok(())
    }
}
