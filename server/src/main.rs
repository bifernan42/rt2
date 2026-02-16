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
