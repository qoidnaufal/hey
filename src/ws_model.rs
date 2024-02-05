use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::auth_model::{ConnectedUser, Status, UserData};

#[derive(Debug, Deserialize)]
struct Msg {
    message: String,
}

pub async fn ws_connection(
    ws: WebSocket,
    email: String,
    connected_user: ConnectedUser,
    user_data: UserData,
) {
    let mut user_state = connected_user
        .write()
        .unwrap()
        .get_mut(&email)
        .unwrap()
        .clone();

    println!(
        "[INFO] New client: {} is {:?}",
        email,
        user_state.status.clone()
    );

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

    user_state.sender = Some(tx);

    let user_name = user_data.user_name;

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

        broadcast_msg(message, &connected_user, &email).await;
    }

    user_state.status = Status::Disconnected;

    println!("[INFO] Client: {} is {:?}", email, user_state.status);
}

async fn broadcast_msg(msg: Message, connected_user: &ConnectedUser, email: &String) {
    if let Message::Text(message) = msg {
        for (other_email, user) in connected_user.read().unwrap().iter() {
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
