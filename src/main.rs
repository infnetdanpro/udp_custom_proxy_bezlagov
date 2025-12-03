mod sync_works;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // client_addr > 45.86.156.93:27015
    let server_addr: SocketAddr = "45.86.156.93:27015".parse().unwrap();

    let client_mapping: Arc<RwLock<HashMap<SocketAddr, SocketAddr>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let client_mapping_cloned = client_mapping.clone();

    // echo
    let main_socket = Arc::new(UdpSocket::bind("0.0.0.0:65000").await?);
    println!("Connect to local address: {:?}", main_socket);
    let main_socket_cloned = main_socket.clone();

    let proxy_socket = Arc::new(UdpSocket::bind("0.0.0.0:64000").await?);
    let proxy_socket_cloned = proxy_socket.clone();

    let client_handler = tokio::spawn(async move {
        loop {
            println!("0: client_handler:0");
            let mut buf = [0; 1024];
            println!("0: client_handler:1");
            // получаем от игрового клиента запрос
            match main_socket.recv_from(&mut buf).await {
                Ok((len, client_addr)) => {
                    println!("0: client_handler:2");
                    // пересылаем на удаленный сервер
                    let mut mapping = client_mapping.write().await;
                    if !mapping.contains_key(&server_addr) {
                        mapping.insert(server_addr, client_addr);
                    }

                    match proxy_socket.send_to(&buf[..len], server_addr).await {
                        Ok(x) => {
                            println!("0: client_handler:2:1");
                            println!("Ok sending {:?}", x);
                        }
                        Err(e) => {
                            println!("0: client_handler:2:2");
                            println!("Error sending: {:?}", e);
                        }
                    };
                }
                Err(e) => println!("{:?}", e),
            }
            println!("0: client_handler:---");
        }
    });

    let server_handler = tokio::spawn(async move {
        loop {
            println!("0: server_handler:0");
            let mut buf = [0; 1024];
            println!("0: server_handler:1");
            // получаем от удаленного сервера
            match proxy_socket_cloned.recv_from(&mut buf).await {
                Ok((len, server_addr)) => {
                    println!("0: server_handler:2");
                    let mapping = client_mapping_cloned.read().await;
                    let client_addr = mapping.get(&server_addr).unwrap();
                    // пересылаем на игровой клиент
                    match main_socket_cloned.send_to(&buf[..len], client_addr).await {
                        Ok(x) => {
                            println!("Ok sending {:?}", x);
                        }
                        Err(e) => {
                            println!("Error sending: {:?}", e);
                        }
                    };
                }
                Err(e) => println!("Error server_handler: {:?}", e),
            };
            println!("0: server_handler:-----");
        }
    });

    tokio::select! {
        _ = client_handler => {},
        _ = server_handler => {},
    }

    Ok(())
}
