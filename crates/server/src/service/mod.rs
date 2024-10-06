pub mod template_service;

#[cfg(not(feature = "play-redis"))]
pub mod redis_fake_service;
#[cfg(not(feature = "play-py-tpl"))]
pub mod tpl_fake_engine;


