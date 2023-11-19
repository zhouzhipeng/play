
#[cfg(ENV = "dev")]
pub const HOST: &str = "http://localhost:3000";
#[cfg(ENV = "prod")]
pub const HOST: &str = "http://127.0.0.1:3000";


///routers
//api
pub const API_ARTICLE_ADD: &str = "/api/article/add";
pub const API_ARTICLE_LIST: &str = "/api/article/list";
