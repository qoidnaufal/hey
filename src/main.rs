use std::{
    collections::HashMap,
    // path::PathBuf,
    sync::{Arc, RwLock},
};

use axum::{
    extract::ws::Message,
    routing::{get, post},
    Extension, Router,
};

use tokio::sync::mpsc;
// use tower_http::services::ServeDir;

mod routes;
mod ws_model;

type RegisteredUsers = Arc<RwLock<HashMap<String, User>>>;

#[derive(Clone)]
pub struct User {
    pub user_name: String,
    pub password: String,
    pub sender: Option<mpsc::UnboundedSender<Message>>,
}

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let registered_users = RegisteredUsers::default();

    // let directory = PathBuf::from("./templates");

    let router = Router::new()
        .route("/", get(routes::login_register_handler))
        .route("/health", get(routes::health_handler))
        .route("/register", post(routes::register_handler))
        .route("/:uuid", get(routes::ws_handler))
        .layer(Extension(registered_users));
    // .fallback_service(ServeDir::new(directory));

    Ok(router.into())
}
