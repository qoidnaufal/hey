use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::extract::{ws::Message, FromRef};
use axum_extra::extract::cookie::Key;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use validator::Validate;

use crate::db;

pub type ConnectedUser = Arc<RwLock<HashMap<String, UserState>>>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum Status {
    Connected,
    #[default]
    Disconnected,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct UserData {
    pub user_name: String,
    pub uuid: String,
    pub email: String,
    pub password: String,
}

#[derive(Clone, Default)]
pub struct UserState {
    pub status: Status,
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

#[derive(Deserialize, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Clone)]
pub struct AppState {
    pub db: db::Database,
    pub user_con: ConnectedUser,
}

#[derive(Clone)]
pub struct CookieKey {
    pub key: Key,
}

impl FromRef<CookieKey> for Key {
    fn from_ref(input: &CookieKey) -> Self {
        input.key.clone()
    }
}
