use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::auth_model::{ConnectedUser, Status};

#[derive(Debug, Deserialize)]
struct Msg {
    message: String,
}

pub async fn ws_connection(ws: WebSocket, email: String, connected_user: ConnectedUser) {
    let mut user_state = connected_user
        .write()
        .unwrap()
        .get_mut(&email)
        .unwrap()
        .clone();
    println!("[INF] New client: {} is {:?}", email, user_state.status);

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
    connected_user
        .write()
        .unwrap()
        .insert(email.clone(), user_state.clone())
        .unwrap();

    while let Some(Ok(message)) = receiver.next().await {
        let message = match message {
            Message::Text(msg) => {
                let deserialized_msg = serde_json::from_str::<Msg>(&msg)
                    .map_err(|err| eprintln!("Unable to deserialize the message: {}", err))
                    .unwrap();

                println!(
                    "[RCV] received message from: {} \n[MSG] {}",
                    user_state.user_name, deserialized_msg.message
                );

                let message_template = format!("<div id='recvchat' hx-swap-oob='beforeend:#log'><p id='username'>{}</p><p>{}</p></div>",
                    user_state.user_name,
                    deserialized_msg.message
                );

                Message::Text(message_template)
            }
            _ => message,
        };

        broadcast_msg(message, &email, &connected_user).await;
    }

    user_state.status = Status::Disconnected;
    connected_user
        .write()
        .unwrap()
        .insert(email.clone(), user_state.clone());

    println!("[INF] Client: {} is {:?}", email, user_state.status);
}

async fn broadcast_msg(msg: Message, email: &String, connected_user: &ConnectedUser) {
    if let Message::Text(message) = msg {
        for (_, user) in connected_user.read().unwrap().iter() {
            if let Some(tx) = user.sender.clone() {
                // this is what being sent back from the server
                match tx.send(Message::Text(message.clone())) {
                    Ok(_) => (),
                    Err(err) => {
                        eprintln!("Unable to send message from: {}, : {}", email, err)
                    }
                }
            }
        }
    }
}
