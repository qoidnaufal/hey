use crate::{
    auth_model::{CookieState, LoginRequest, RegisterRequest, RegisteredUsers, Status},
    page_template::{ChatPage, LoginPage, LoginRegisterResponse, MyChat, RegisterPage},
    ws_model::{register_user, ws_connection},
};
use axum::{
    extract::{Json, Path, State, WebSocketUpgrade},
    http::{Response, StatusCode},
    response::{AppendHeaders, IntoResponse},
    Extension,
};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar};
use uuid::Uuid;
use validator::Validate;

// ------

pub async fn register_page() -> impl IntoResponse {
    RegisterPage
}

pub async fn login_page() -> impl IntoResponse {
    LoginPage
}

pub async fn get_chat_page(
    jar: PrivateCookieJar,
    Extension(registered_users): Extension<RegisteredUsers>,
) -> impl IntoResponse {
    match jar.get("login_id").map(|cookie| cookie.value().to_owned()) {
        Some(email) => match registered_users.write().unwrap().get_mut(&email) {
            Some(user) => {
                user.status = Status::Connected;
                ChatPage { email }.into_response()
            }
            None => RegisterPage.into_response(),
        },
        None => RegisterPage.into_response(),
    }
}

pub async fn register_handler(
    Extension(registered_users): Extension<RegisteredUsers>,
    Json(body): Json<RegisterRequest>,
) -> impl IntoResponse {
    match body.validate() {
        Ok(_) => {
            let user_name = body.user_name;
            let email = body.email;
            let password = body.password;
            let uuid = Uuid::new_v4().as_simple().to_string();

            match register_user(uuid, user_name.clone(), email, password, registered_users).await {
                Ok(_) => resp_builder(
                    StatusCode::CREATED,
                    format!("Hello {}, you've been registered", user_name),
                ),
                Err(err) => resp_builder(StatusCode::UNAUTHORIZED, err),
            }
        }
        Err(err) => {
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

            resp_builder(StatusCode::UNAUTHORIZED, response)
        }
    }
}

pub async fn login_handler(
    Extension(registered_users): Extension<RegisteredUsers>,
    State(state): State<CookieState>,
    Json(body): Json<LoginRequest>,
) -> Result<
    (
        PrivateCookieJar,
        AppendHeaders<[(&'static str, &'static str); 1]>,
    ),
    impl IntoResponse,
> {
    let email = body.email;
    let password = body.password;

    match registered_users.write().unwrap().get_mut(&email) {
        Some(user) => {
            if password == user.password && email == user.email {
                user.status = Status::Connected;

                let jar = PrivateCookieJar::new(state.key);
                let mut cookie = Cookie::new("login_id", email.clone());
                cookie.set_http_only(true);
                cookie.set_secure(true);
                let jar = jar.add(cookie);

                Ok((jar, AppendHeaders([("HX-Redirect", "/")])))
            } else {
                Err(resp_builder(
                    StatusCode::UNAUTHORIZED,
                    "Failed to login. The username, email and password didn't match".to_string(),
                )
                .into_response())
            }
        }
        None => Err(resp_builder(
            StatusCode::UNAUTHORIZED,
            "You're not yet registered. Register now!".to_string(),
        )
        .into_response()),
    }
}

fn resp_builder(status: StatusCode, response: String) -> impl IntoResponse {
    Response::builder()
        .status(status)
        .body(LoginRegisterResponse { response }.into_response())
        .unwrap()
        .into_response()
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

pub async fn my_chat(Json(body): Json<MyChat>) -> impl IntoResponse {
    MyChat {
        message: body.message,
    }
}
