use std::{net::SocketAddr, path::PathBuf};

use axum::{
    handler::HandlerWithoutStateExt,
    routing::{get, post},
    Extension, Router,
};

use axum_extra::extract::cookie::Key;
use axum_server::tls_rustls::RustlsConfig;

mod auth_model;
mod db;
mod page_template;
mod routes;
mod ws_model;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let app_state = auth_model::AppState {
        db: db::Database::init("hey".to_string(), "users".to_string())
            .await
            .map_err(|err| eprintln!("{}", err))
            .unwrap(),
        user_con: auth_model::ConnectedUser::default(),
    };

    let cookie_key = auth_model::CookieKey {
        key: Key::generate(),
    };

    let config = RustlsConfig::from_pem_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self-signed-certs")
            .join("certificate.pem"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("self-signed-certs")
            .join("key.pem"),
    )
    .await?;

    let router = Router::new()
        .route("/register", post(routes::register_handler))
        .route("/registerpage", get(routes::register_page))
        .route("/login", post(routes::login_handler))
        .route("/loginpage", get(routes::login_page))
        .route("/", get(routes::get_chat_page))
        .route("/ws/:email", get(routes::ws_handler))
        .route("/mychat", post(routes::my_chat))
        .layer(Extension(app_state))
        .with_state(cookie_key)
        .fallback_service(routes::register_page.into_service());

    let addr = SocketAddr::from(([0, 0, 0, 0], 6969));

    axum_server::bind_rustls(addr, config)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
