use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::auth_model::{RegisteredUsers, Status, User};

#[derive(Debug, Deserialize)]
struct Msg {
    message: String,
}

pub async fn ws_connection(
    ws: WebSocket,
    email: String,
    registered_users: RegisteredUsers,
    mut user: User,
) {
    println!("[INFO] New client: {} is {:?}", email, user.status);

    let (mut sender, mut receiver) = ws.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match sender.send(msg).await {
                Ok(_) => (),
                Err(err) => eprintln!("Unable to send ws message: {}", err),
            }
        }
        sender.close().await.unwrap();
    });

    user.sender = Some(tx);

    registered_users
        .write()
        .unwrap()
        .insert(email.clone(), user.clone());

    let user_name = registered_users
        .read()
        .unwrap()
        .get(&email)
        .unwrap()
        .user_name
        .clone();

    while let Some(Ok(message)) = receiver.next().await {
        let message = match message {
            Message::Text(msg) => {
                let deserialized_msg = serde_json::from_str::<Msg>(&msg)
                    .map_err(|err| eprintln!("Unable to deserialize the message: {}", err))
                    .unwrap();

                println!("[MESSAGE] {} send: {}", user_name, deserialized_msg.message);

                let message_template = format!("<div id='recvchat' hx-swap-oob='beforeend:#log'><p id='username'>{}</p><p>{}</p></div>",
                    user_name,
                    deserialized_msg.message
                );

                Message::Text(message_template)
            }
            _ => message,
        };

        broadcast_msg(message, &registered_users, &email).await;
    }

    user.status = Status::Disconnected;
    registered_users
        .write()
        .unwrap()
        .insert(email.clone(), user.clone());

    println!("[INFO] Client: {} is {:?}", email, user.status);
}

async fn broadcast_msg(msg: Message, registered_users: &RegisteredUsers, email: &String) {
    if let Message::Text(message) = msg {
        for (other_email, user) in registered_users.read().unwrap().iter() {
            if let (Status::Connected, Some(tx)) = (user.status.clone(), user.sender.clone()) {
                if email != other_email {
                    match tx.send(Message::Text(message.clone())) {
                        Ok(_) => (),
                        Err(err) => eprintln!("Unable to send message from: {}, : {}", email, err),
                    }
                }
            }
        }
    }
}

pub async fn register_user(
    uuid: String,
    user_name: String,
    email: String,
    password: String,
    registered_users: RegisteredUsers,
) -> Result<(), String> {
    if registered_users.read().unwrap().get(&email).is_none() {
        match registered_users.write().unwrap().insert(
            email.clone(),
            User {
                status: Status::default(),
                uuid,
                user_name,
                email,
                password,
                sender: None,
            },
        ) {
            None => Ok(()),
            Some(n) => Err(format!(
                "User with email \"{}\" is already registered",
                n.email
            )),
        }
    } else {
        Err(format!(
            "User with email \"{}\" is already registered",
            email
        ))
    }
}
