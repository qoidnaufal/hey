use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::extract::{ws::Message, FromRef};
use axum_extra::extract::cookie::Key;
use serde::Deserialize;
use tokio::sync::mpsc::UnboundedSender;
use validator::Validate;

pub type RegisteredUsers = Arc<RwLock<HashMap<String, User>>>;

#[derive(Clone, Debug, Default)]
pub enum Status {
    Connected,
    #[default]
    Disconnected,
}

#[derive(Clone)]
pub struct User {
    pub status: Status,
    pub uuid: String,
    pub user_name: String,
    pub email: String,
    pub password: String,
    pub sender: Option<UnboundedSender<Message>>,
}

#[derive(Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(length(min = 1, message = "don't you have a name?"))]
    pub user_name: String,
    #[validate(email(message = "use valid email"))]
    pub email: String,
    #[validate(length(min = 12, message = "password need to be at least 12 characters"))]
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Clone)]
pub struct CookieState {
    pub key: Key,
}

impl FromRef<CookieState> for Key {
    fn from_ref(input: &CookieState) -> Self {
        input.key.clone()
    }
}
