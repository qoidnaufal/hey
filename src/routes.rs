use crate::{
    auth_model::{AppState, CookieKey, LoginRequest, RegisterRequest, Status, UserData, UserState},
    page_template::{ChatPage, LoginPage, LoginRegisterResponse, MyChat, RegisterPage},
    ws_model::ws_connection,
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

#[axum_macros::debug_handler]
pub async fn get_chat_page(
    jar: PrivateCookieJar,
    State(_): State<CookieKey>,
    Extension(app_state): Extension<AppState>,
) -> impl IntoResponse {
    match jar.get("user_id").map(|cookie| cookie.value().to_owned()) {
        Some(uuid) => match app_state.db.get_user_by_id("user_data", uuid.clone()).await {
            Some(user_data) => match app_state.user_con.write() {
                Ok(mut connected_user) => match connected_user.get_mut(&user_data.email) {
                    Some(user_con) => {
                        user_con.status = Status::Connected;
                        ChatPage {
                            email: user_data.email,
                        }
                        .into_response()
                    }
                    None => {
                        connected_user
                            .insert(
                                uuid,
                                UserState {
                                    status: Status::Connected,
                                    sender: None,
                                },
                            )
                            .unwrap();
                        ChatPage {
                            email: user_data.email,
                        }
                        .into_response()
                    }
                },
                Err(err) => {
                    eprintln!("Unable to access the hashmap: {}", err);
                    RegisterPage.into_response()
                }
            },
            None => RegisterPage.into_response(),
        },
        None => RegisterPage.into_response(),
    }
}

pub async fn register_handler(
    Extension(app_state): Extension<AppState>,
    Json(body): Json<RegisterRequest>,
) -> impl IntoResponse {
    match body.validate() {
        Ok(_) => {
            let user_name = body.user_name;
            let email = body.email;
            let password = body.password;
            let uuid = Uuid::new_v4().as_simple().to_string();

            let new_user = UserData {
                user_name: user_name.clone(),
                uuid: uuid.clone(),
                email,
                password,
            };

            match app_state
                .db
                .register_user("user_data", uuid, new_user)
                .await
            {
                Ok(_) => resp_builder(
                    StatusCode::CREATED,
                    format!("Hello {}, you've been registered", user_name),
                ),
                Err(err) => resp_builder(
                    StatusCode::UNAUTHORIZED,
                    format!("Failed to register: {}", err),
                ),
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
    Extension(app_state): Extension<AppState>,
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
    let email = body.email;
    let password = body.password;

    match app_state
        .db
        .get_user_by_query("user_data", email.clone(), password)
        .await
    {
        Ok(Some(uuid)) => {
            if let Some(user_data) = app_state.db.get_user_by_id("user_data", uuid.clone()).await {
                match app_state.user_con.write().unwrap().insert(
                    user_data.email,
                    UserState {
                        status: Status::Connected,
                        sender: None,
                    },
                ) {
                    None => {
                        let mut cookie = Cookie::new("user_id", uuid);
                        cookie.set_http_only(true);
                        cookie.set_secure(true);
                        let jar = jar.add(cookie);

                        Ok((jar, AppendHeaders([("HX-Redirect", "/")])))
                    }
                    Some(_) => Err(resp_builder(
                        StatusCode::CONFLICT,
                        "Looks like you're already logged in".to_string(),
                    )
                    .into_response()),
                }
            } else {
                Err(resp_builder(
                    StatusCode::UNAUTHORIZED,
                    "Failed to login. The username, email and password didn't match".to_string(),
                )
                .into_response())
            }
        }
        Ok(None) => Err(resp_builder(
            StatusCode::UNAUTHORIZED,
            "You're not yet registered. Register Now!".to_string(),
        )
        .into_response()),
        Err(err) => {
            Err(resp_builder(StatusCode::UNAUTHORIZED, format!("{:?}", err)).into_response())
        }
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
    jar: PrivateCookieJar,
    State(_): State<CookieKey>,
    Extension(app_state): Extension<AppState>,
    Path(email): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    match jar.get("user_id").map(|cookie| cookie.value().to_owned()) {
        Some(user_id) => {
            match app_state.db.get_user_by_id("user_data", user_id).await {
                Some(user_data) => Ok(ws.on_upgrade(|socket| {
                    ws_connection(socket, email, app_state.user_con, user_data)
                })),
                None => Err(StatusCode::BAD_REQUEST),
            }
        }
        None => Err(StatusCode::BAD_REQUEST),
    }
}

pub async fn my_chat(Json(body): Json<MyChat>) -> impl IntoResponse {
    MyChat {
        message: body.message,
    }
}
