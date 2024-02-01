use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::{
    extract::ws::Message,
    handler::HandlerWithoutStateExt,
    routing::{get, post},
    Extension, Router,
};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

mod routes;
mod ws_model;

type RegisteredUsers = Arc<RwLock<HashMap<String, User>>>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub enum Status {
    LoggedIN,
    #[default]
    LoggedOUT,
}

#[derive(Clone)]
pub struct User {
    pub status: Status,
    pub uuid: String,
    pub user_name: String,
    pub email: String,
    pub password: String,
    pub sender: Option<mpsc::UnboundedSender<Message>>,
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let registered_users = RegisteredUsers::default();

    let router = Router::new()
        .route("/register", post(routes::register_handler))
        .route("/login", post(routes::login_handler))
        .route("/", get(routes::get_chat_page))
        .route("/ws/:email", get(routes::ws_handler))
        .layer(Extension(registered_users))
        .fallback_service(routes::login_register_page.into_service());

    Ok(router.into())
}
