use futures::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast::{self, Sender},
};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

#[path = "shared/lib.rs"]
mod lib;
use lib::random_name;

const HELP_MSG: &str = include_str!("shared/help-01.txt");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    let (tx, _) = broadcast::channel::<String>(32);
    loop {
        let (tcp, _) = server.accept().await?;
        tokio::spawn(handle_user(tcp, tx.clone()));
    }
}

async fn handle_user(mut tcp: TcpStream, tx: Sender<String>) -> anyhow::Result<()> {
    let (reader, writer) = tcp.split();
    let mut stream = FramedRead::new(reader, LinesCodec::new());
    let mut sink = FramedWrite::new(writer, LinesCodec::new());
    let mut rx = tx.subscribe();
    let name = random_name();
    sink.send(HELP_MSG).await?;
    sink.send(format!("You are {name}")).await?;
    loop {
        tokio::select! {
            user_msg = stream.next() => {
                let user_msg = match user_msg {
                    Some(msg) => msg?,
                    None => break,
                };
                if user_msg.starts_with("/help") {
                    sink.send(HELP_MSG).await?;
                } else if user_msg.starts_with("/quit") {
                    break;
                } else {
                    tx.send(format!("{name}: {user_msg}"))?;
                }
            },
            peer_msg = rx.recv() => {
                sink.send(peer_msg?).await?;
            },
        }
    }
    Ok(())
}

/*
use std::{
    collections::HashMap,
    net::{SocketAddr, UdpSocket},
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
enum NetMessage {
    Join { username: String },
    Leave { username: String },
    Chat { username: String, content: String },
}

fn broadcast(
    socket: &UdpSocket,
    clients: &HashMap<SocketAddr, String>,
    sender: SocketAddr,
    message: NetMessage,
) {
    let data = serde_json::to_vec(&message).unwrap();

    for (addr, _) in clients {
        if *addr != sender {
            let _ = socket.send_to(&data, addr);
        }
    }
}

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:8080").unwrap();
    println!("UDP Server running on 8080");

    let mut clients: HashMap<SocketAddr, String> = HashMap::new();
    let mut buf = [0u8; 2048];

    loop {
        let (size, addr) = socket.recv_from(&mut buf).unwrap();

        if let Ok(msg) = serde_json::from_slice::<NetMessage>(&buf[..size]) {
            match msg {
                NetMessage::Join { username } => {
                    println!("{username} joined from {addr}");
                    clients.insert(addr, username.clone());

                    broadcast(&socket, &clients, addr, NetMessage::Join { username });
                }

                NetMessage::Chat { username, content } => {
                    broadcast(
                        &socket,
                        &clients,
                        addr,
                        NetMessage::Chat { username, content },
                    );
                }

                NetMessage::Leave { username } => {
                    println!("{username} left");
                    clients.remove(&addr);

                    broadcast(&socket, &clients, addr, NetMessage::Leave { username });
                }
            }
        }
    }
}
*/
