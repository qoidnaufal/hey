use crate::{
    ws_model::{register_user, ws_connection},
    RegisteredUsers,
};
use askama::Template;
use axum::{
    extract::{Json, Path, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
    Extension,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct LoginRegisterRequest {
    user_name: String,
    password: String,
}

#[derive(Template)]
#[template(path = "registerresponse.html")]
pub struct RegisterResponse {
    uuid: String,
    user_name: String,
}

#[derive(Template)]
#[template(path = "loginregister.html")]
pub struct LoginRegister;

pub async fn login_register_handler() -> impl IntoResponse {
    LoginRegister
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(registered_users): Extension<RegisteredUsers>,
    Path(uuid): Path<String>,
) -> impl IntoResponse {
    let user = registered_users.read().unwrap().get(&uuid).unwrap().clone();
    ws.on_upgrade(|socket| ws_connection(socket, uuid, registered_users, user))
}

pub async fn register_handler(
    Extension(registered_users): Extension<RegisteredUsers>,
    Json(body): Json<LoginRegisterRequest>,
) -> impl IntoResponse {
    let user_name = body.user_name;
    let password = body.password;
    let uuid = Uuid::new_v4().as_simple().to_string();

    register_user(user_name.clone(), password, uuid.clone(), registered_users).await;

    RegisterResponse { uuid, user_name }
}

pub async fn login_handler(
    Extension(registered_users): Extension<RegisteredUsers>,
    Json(body): Json<LoginRegisterRequest>,
) -> impl IntoResponse {
    let user_name = body.user_name;
    let password = body.password;
}

pub async fn health_handler() -> impl IntoResponse {
    StatusCode::OK
}
