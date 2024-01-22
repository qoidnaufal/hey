use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::{RegisteredUsers, User};

pub async fn ws_connection(
    ws: WebSocket,
    uuid: String,
    registered_users: RegisteredUsers,
    mut user: User,
) {
    let (mut sender, mut receiver) = ws.split();

    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let _ = sender
                .send(msg)
                .await
                .map_err(|err| eprintln!("Unable to send message from the sender: {}", err));
        }
        sender.close().await.unwrap();
    });

    user.sender = Some(tx);
    registered_users.write().unwrap().insert(uuid.clone(), user);

    while let Some(Ok(result)) = receiver.next().await {
        println!("{:?}", result);
        broadcast_msg(result, &registered_users).await;
    }

    registered_users.write().unwrap().remove(&uuid);
}

pub async fn register_user(
    user_name: String,
    password: String,
    uuid: String,
    registered_users: RegisteredUsers,
) {
    registered_users.write().unwrap().insert(
        uuid,
        User {
            user_name,
            password,
            sender: None,
        },
    );
}

pub async fn broadcast_msg(msg: Message, registered_users: &RegisteredUsers) {
    if let Message::Text(message) = msg {
        for (uuid, user) in registered_users.read().unwrap().iter() {
            if let Some(tx) = user.sender.clone() {
                match tx.send(Message::Text(message.clone())) {
                    Ok(_) => (),
                    Err(err) => eprintln!("Unable to send message from: {}, : {}", uuid, err),
                }
            }
        }
    }
}
