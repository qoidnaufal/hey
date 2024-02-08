use std::sync::Arc;

use crate::{
    auth_model::{AppState, CookieKey, LoginRequest, RegisterRequest, Status, UserData, UserState},
    page_template::{ChatPage, LoginPage, LoginRegisterResponse, MyChat, RegisterPage},
    ws_model::ws_connection,
};
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    extract::{Json, State, WebSocketUpgrade},
    http::{Response, StatusCode},
    response::{AppendHeaders, IntoResponse},
    Extension,
};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar};
use rand_core::OsRng;
use uuid::Uuid;
use validator::Validate;

pub async fn register_page() -> impl IntoResponse {
    RegisterPage
}

pub async fn login_page() -> impl IntoResponse {
    LoginPage
}

pub async fn get_chat_page(
    jar: PrivateCookieJar,
    State(_): State<CookieKey>,
    Extension(app_state): Extension<Arc<AppState>>,
) -> impl IntoResponse {
    match jar.get("user_id").map(|cookie| cookie.value().to_owned()) {
        Some(uuid) => match app_state.db.get_user_by_id(&uuid).await {
            Some(_user_data) => ChatPage.into_response(),
            None => RegisterPage.into_response(),
        },
        None => RegisterPage.into_response(),
    }
}

pub async fn register_handler(
    Extension(app_state): Extension<Arc<AppState>>,
    Json(body): Json<RegisterRequest>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    match body.validate() {
        Ok(_) => {
            let salt = SaltString::generate(&mut OsRng);
            let password = Argon2::default()
                .hash_password(body.password.as_bytes(), &salt)
                .map_err(|err| {
                    resp_builder(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unable to hash password: {}", err),
                    )
                })
                .map(|pwd| pwd.to_string())?;

            let user_name = body.user_name;
            let email = body.email.to_ascii_lowercase();
            let uuid = Uuid::new_v4().as_simple().to_string();

            let new_user = UserData {
                user_name: user_name.clone(),
                uuid: uuid.clone(),
                email: email.clone(),
                password: password.clone(),
            };

            match app_state.db.get_user_by_email(email.clone()).await {
                Ok(Some(_)) => Err(resp_builder(
                    StatusCode::CONFLICT,
                    format!("User with this email: {} is already registered", email),
                )),
                Ok(None) => match app_state.db.register_user(uuid, new_user).await {
                    Ok(_) => Ok(resp_builder(
                        StatusCode::CREATED,
                        "Registration successful, you can login now!".to_string(),
                    )),
                    Err(err) => Err(resp_builder(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Registration failed: {}", err),
                    )),
                },
                Err(err) => Err(resp_builder(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Something is wrong, try again later: {}", err),
                )),
            }
        }
        Err(err) => {
            // this error handling is a bit verbose
            // because i can't validate on the client side
            // later i'll change the ui component to Leptos

            let err_msg = err
                .field_errors()
                .into_values()
                .map(|e| match e[0].message.clone() {
                    Some(message) => message,
                    None => std::borrow::Cow::Borrowed(""),
                })
                .collect::<Vec<_>>();

            let response = match err_msg.len() {
                1 => format!("Invalid input: {}!", err_msg[0]),
                2 => format!("Invalid input: {} & {}!", err_msg[0], err_msg[1]),
                3 => format!(
                    "Invalid input: {} & {} & {}!",
                    err_msg[0], err_msg[1], err_msg[2]
                ),
                _ => "".to_string(),
            };

            Err(resp_builder(StatusCode::UNAUTHORIZED, response))
        }
    }
}

pub async fn login_handler(
    Extension(app_state): Extension<Arc<AppState>>,
    jar: PrivateCookieJar,
    State(_): State<CookieKey>,
    Json(body): Json<LoginRequest>,
) -> Result<
    (
        PrivateCookieJar,
        AppendHeaders<[(&'static str, &'static str); 1]>,
    ),
    impl IntoResponse,
> {
    match app_state.db.get_user_by_email(body.email.clone()).await {
        Ok(Some(user_data)) => match PasswordHash::new(&user_data.password) {
            Ok(parsed_password) => match Argon2::default()
                .verify_password(body.password.as_bytes(), &parsed_password)
            {
                Ok(_) => {
                    let cookie = Cookie::build(("user_id", user_data.uuid))
                        .http_only(true)
                        .path("/")
                        .same_site(axum_extra::extract::cookie::SameSite::Lax)
                        .secure(true);
                    let jar = jar.add(cookie);

                    Ok((jar, AppendHeaders([("HX-Redirect", "/")])))
                }
                Err(_) => Err(resp_builder(
                    StatusCode::UNAUTHORIZED,
                    "Failed to login. The email and password didn't match",
                )
                .into_response()),
            },
            Err(err) => Err(resp_builder(
                StatusCode::UNAUTHORIZED,
                format!("Unable to parse the password: {}", err),
            )
            .into_response()),
        },
        Ok(None) => Err(resp_builder(
            StatusCode::UNAUTHORIZED,
            "You're not yet registered. Register Now!",
        )
        .into_response()),
        Err(err) => {
            Err(resp_builder(StatusCode::UNAUTHORIZED, format!("{:?}", err)).into_response())
        }
    }
}

fn resp_builder<S: std::fmt::Display>(status: StatusCode, response: S) -> impl IntoResponse {
    Response::builder()
        .status(status)
        .body(LoginRegisterResponse { response }.into_response())
        .unwrap()
        .into_response()
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    jar: PrivateCookieJar,
    State(_): State<CookieKey>,
    Extension(app_state): Extension<Arc<AppState>>,
) -> Result<impl IntoResponse, StatusCode> {
    match jar.get("user_id").map(|cookie| cookie.value().to_owned()) {
        Some(user_id) => match app_state.db.get_user_by_id(&user_id).await {
            Some(user_data) => {
                let user_state = UserState {
                    user_name: user_data.user_name,
                    uuid: user_data.uuid,
                    status: Status::Connected,
                    sender: None,
                };
                match app_state
                    .con
                    .clone()
                    .write()
                    .unwrap()
                    .insert(user_data.email.clone(), user_state)
                {
                    _ => Ok(ws.on_upgrade(move |socket| {
                        ws_connection(socket, user_data.email, app_state.con.clone())
                    })),
                }
            }
            None => Err(StatusCode::BAD_REQUEST),
        },
        None => Err(StatusCode::BAD_REQUEST),
    }
}

pub async fn my_chat(Json(body): Json<MyChat>) -> impl IntoResponse {
    MyChat {
        message: body.message,
    }
}
