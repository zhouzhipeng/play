pub mod template_service;

#[cfg(not(feature = "redis"))]
pub mod redis_fake_service;
#[cfg(not(feature = "tpl"))]
pub mod tpl_fake_engine;
mod openai_service;


