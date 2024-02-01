use crate::{
    ws_model::{register_user, ws_connection},
    RegisteredUsers, Status,
};
use askama::Template;
use axum::{
    extract::{Json, Path, WebSocketUpgrade},
    http::{header::SET_COOKIE, Response},
    response::IntoResponse,
    Extension,
};
#[allow(unused_imports)]
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ------

#[derive(Deserialize, Validate)]
pub struct LoginRegisterRequest {
    #[validate(length(min = 1, message = "don't you have a name?"))]
    user_name: String,
    #[validate(email(message = "use valid email"))]
    email: String,
    #[validate(length(min = 12, message = "password need to be at least 12 characters"))]
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

// ------

pub async fn login_register_page() -> impl IntoResponse {
    LoginRegisterPage
}

pub async fn get_chat_page(
    jar: CookieJar,
    Extension(registered_users): Extension<RegisteredUsers>,
) -> impl IntoResponse {
    match jar.get("email").map(|cookie| cookie.value().to_owned()) {
        Some(email) => match registered_users.write().unwrap().get_mut(&email) {
            Some(user) => {
                user.status = Status::LoggedIN;
                ChatPage { email }.into_response()
            }
            None => LoginRegisterPage.into_response(),
        },
        None => LoginRegisterPage.into_response(),
    }
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
    match body.validate() {
        Ok(_) => {
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
        Err(err) => {
            let err_msg = err
                .field_errors()
                .into_values()
                .map(|e| match e[0].message.clone() {
                    Some(n) => n,
                    None => std::borrow::Cow::Borrowed(""),
                })
                .collect::<Vec<_>>();

            let response = match err_msg.len() {
                1 => format!("Invalid input: {}", err_msg[0]),
                2 => format!("Invalid input: {} & {}", err_msg[0], err_msg[1]),
                3 => format!(
                    "Invalid input: {} & {} & {}",
                    err_msg[0], err_msg[1], err_msg[2]
                ),
                _ => "".to_string(),
            };

            Response::builder()
                .status(409)
                .body(LoginRegisterResponse { response }.into_response())
                .unwrap()
        }
    }
}

pub async fn login_handler(
    Extension(registered_users): Extension<RegisteredUsers>,
    Json(body): Json<LoginRegisterRequest>,
) -> impl IntoResponse {
    let user_name = body.user_name;
    let email = body.email;
    let password = body.password;

    match registered_users.write().unwrap().get_mut(&email) {
        Some(user) => {
            if user_name == user.user_name && password == user.password && email == user.email {
                user.status = Status::LoggedIN;
                Response::builder()
                    .status(303)
                    .header("HX-Redirect", "")
                    .header(SET_COOKIE, format!("email={}", user.email))
                    .body(ChatPage { email }.into_response())
                    .unwrap()
                    .into_response()
            } else {
                login_error(
                    "Failed to login. The username, email and password didn't match".to_string(),
                )
                .into_response()
            }
        }
        None => login_error("You're not yet registered. Register now!".to_string()).into_response(),
    }
}

fn login_error(response: String) -> impl IntoResponse {
    Response::builder()
        .status(401)
        .body(LoginRegisterResponse { response }.into_response())
        .unwrap()
        .into_response()
}
