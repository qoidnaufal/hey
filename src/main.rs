use std::{net::SocketAddr, path::PathBuf};

use axum::{
    handler::HandlerWithoutStateExt,
    routing::{get, post},
    Extension, Router,
};

use axum_extra::extract::cookie::Key;
use axum_server::tls_rustls::RustlsConfig;

mod auth_model;
mod page_template;
mod routes;
mod ws_model;

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let registered_users = auth_model::RegisteredUsers::default();

    let state = auth_model::CookieState {
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
        .layer(Extension(registered_users))
        .with_state(state)
        .fallback_service(routes::register_page.into_service());

    let addr = SocketAddr::from(([0, 0, 0, 0], 6969));

    axum_server::bind_rustls(addr, config)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}
