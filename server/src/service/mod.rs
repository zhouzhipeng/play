pub mod template_service;

#[cfg(not(feature = "redis"))]
pub mod redis_fake_service;

