use askama::Template;
use serde::Deserialize;

#[derive(Template)]
#[template(path = "loginregisterresponse.html")]
pub struct LoginRegisterResponse<S: std::fmt::Display> {
    pub response: S,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct ChatPage;

#[derive(Template)]
#[template(path = "registerpage.html")]
pub struct RegisterPage;

#[derive(Template)]
#[template(path = "loginpage.html")]
pub struct LoginPage;

#[derive(Template, Deserialize, Debug)]
#[template(path = "mychat.html")]
pub struct MyChat {
    pub message: String,
}
