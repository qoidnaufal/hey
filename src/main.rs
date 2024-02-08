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
    let app_state = std::sync::Arc::new(auth_model::AppState {
        db: db::Database::init()
            .await
            .map_err(|err| std::io::Error::other(format!("{}", err)))?,
        con: auth_model::ConnectedUser::default(),
    });

    let key = include_str!("../secrets/key.txt")
        .trim()
        .split(",")
        .map(|str| str.parse::<u8>().unwrap())
        .collect::<Vec<_>>();

    let key = Key::from(key.as_slice());

    let cookie_key = auth_model::CookieKey { key };

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
        .route("/ws", get(routes::ws_handler))
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
