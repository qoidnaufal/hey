use crate::{
    ws_model::{register_user, ws_connection},
    RegisteredUsers,
};
use askama::Template;
use axum::{
    extract::{Json, Path, WebSocketUpgrade},
    http::Response,
    response::IntoResponse,
    Extension,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct LoginRegisterRequest {
    user_name: String,
    email: String,
    password: String,
}

#[derive(Template)]
#[template(path = "loginregisterresponse.html")]
pub struct LoginRegisterResponse {
    response: String,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct ChatPage {
    email: String,
}

#[derive(Template)]
#[template(path = "loginregister.html")]
pub struct LoginRegisterPage;

pub async fn login_register_page() -> impl IntoResponse {
    LoginRegisterPage
}

pub async fn get_chat_page(Path(email): Path<String>) -> impl IntoResponse {
    ChatPage { email }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(registered_users): Extension<RegisteredUsers>,
    Path(email): Path<String>,
) -> impl IntoResponse {
    let user = registered_users
        .read()
        .unwrap()
        .get(&email)
        .unwrap()
        .clone();

    ws.on_upgrade(|socket| ws_connection(socket, email, registered_users, user))
}

pub async fn register_handler(
    Extension(registered_users): Extension<RegisteredUsers>,
    Json(body): Json<LoginRegisterRequest>,
) -> impl IntoResponse {
    let user_name = body.user_name;
    let email = body.email;
    let password = body.password;
    let uuid = Uuid::new_v4().as_simple().to_string();

    match register_user(uuid, user_name.clone(), email, password, registered_users).await {
        Ok(_) => Response::builder()
            .status(201)
            .body(
                LoginRegisterResponse {
                    response: format!("Hello {}, you've been registered!", user_name),
                }
                .into_response(),
            )
            .unwrap(),
        Err(err) => Response::builder()
            .status(409)
            .body(LoginRegisterResponse { response: err }.into_response())
            .unwrap(),
    }
}

pub async fn login_handler(
    Extension(registered_users): Extension<RegisteredUsers>,
    Json(body): Json<LoginRegisterRequest>,
) -> impl IntoResponse {
    let user_name = body.user_name;
    let email = body.email;
    let password = body.password;

    match registered_users.read().unwrap().get(&email) {
        Some(user) => {
            if user_name == user.user_name && password == user.password && email == user.email {
                Response::builder()
                    .status(303)
                    .header("HX-Redirect", email.clone())
                    .body(ChatPage { email }.into_response())
                    .unwrap()
            } else {
                Response::builder()
                    .status(401)
                    .body(
                        LoginRegisterResponse {
                            response:
                                "Failed to login. The username, email and password didn't match"
                                    .to_string(),
                        }
                        .into_response(),
                    )
                    .unwrap()
            }
        }
        None => Response::builder()
            .status(401)
            .body(
                LoginRegisterResponse {
                    response: "You're not yet registered. Register now!".to_string(),
                }
                .into_response(),
            )
            .unwrap(),
    }
}
