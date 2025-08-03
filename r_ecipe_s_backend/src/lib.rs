#[cfg(feature = "backend")]
pub mod app_config;
// #[cfg(feature = "backend")]
// pub mod auth;
#[cfg(feature = "backend")]
pub mod db;
pub mod model;
#[cfg(feature = "backend")]
pub mod recipe_service;
#[cfg(feature = "backend")]
pub mod repository;

#[cfg(feature = "backend")]
pub mod error;
