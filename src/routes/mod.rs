// src/routes/mod.rs
pub mod health_check;
pub mod subscriptions;
pub mod subscriptions_confirm;

pub use health_check::health_check;
pub use subscriptions::subscribe;
pub use subscriptions_confirm::confirm;
