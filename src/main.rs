mod args;
mod buffers;

use crate::args::parse_args;
use crate::buffers::BufferPool;
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio::time::timeout;

const INACTIVITY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Создаем пул буферов
    let buffer_pool = Arc::new(BufferPool::new());
    let buffer_pool_clone = buffer_pool.clone();

    // client_addr > 45.86.156.93:27015
    // нужен таймаут соединений, допустим, если последний пакет был 3 минуты назад, то дропать маппинг/соединение
    // этот маппинг заранее надо создавать где-то в HTTP интерфейсе
    let res = parse_args();
    if res.is_err() {
        panic!(
            "Error parsing args: {:?}. \nExample: --server_address 45.86.156.93:27015",
            res.err()
        );
    }

    let args = res.unwrap();

    let server_address: SocketAddr = args.server_address.parse().unwrap();
    let bind_address: SocketAddr = args.bind_address.parse().unwrap();

    // Маппинг для роутинга между сервером и клиентом
    let client_mapping: Arc<DashMap<SocketAddr, SocketAddr>> = Arc::new(DashMap::new());
    let client_mapping_cloned = client_mapping.clone();

    // Создаем 2 сокета, один на прием от игрового клиента, второй на общение с игровым сервером
    let game_client_socket = Arc::new(UdpSocket::bind(bind_address).await?);
    println!("Connect to local address: {:?}", game_client_socket);
    let game_client_socket_clone = game_client_socket.clone();

    let game_server_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    let game_server_socket_clone = game_server_socket.clone();

    let (tx, mut rx): (Sender<i32>, Receiver<i32>) = channel(100);
    let tx_clone = tx.clone();

    let client_handler = tokio::spawn(async move {
        loop {
            let mut buf = buffer_pool.get_buffer().await;
            // получаем от игрового клиента запрос
            match game_client_socket.recv_from(&mut buf).await {
                Ok((len, client_addr)) => {
                    // проверяем есть ли маппинг соединения, если нет, то добавляем
                    if !client_mapping.contains_key(&server_address) {
                        client_mapping.insert(server_address, client_addr);
                    }

                    // пересылаем на удаленный сервер
                    match game_server_socket
                        .send_to(&buf[..len], server_address)
                        .await
                    {
                        Ok(_) => {
                            tx.send(1).await.unwrap();
                        }
                        Err(e) => {
                            println!("[game_server_socket] Error sending: {:?}", e);
                        }
                    };
                }
                Err(e) => println!("[game_client_socket] Broad error: {:?}", e),
            }
            buffer_pool.return_buffer(buf).await;
        }
    });

    let server_handler = tokio::spawn(async move {
        loop {
            let mut buf = buffer_pool_clone.get_buffer().await;
            // получаем от удаленного сервера
            match game_server_socket_clone.recv_from(&mut buf).await {
                Ok((len, server_address)) => {
                    let client_addr = client_mapping_cloned.get(&server_address).unwrap();
                    // пересылаем на игровой клиент
                    match game_client_socket_clone
                        .send_to(&buf[..len], *client_addr)
                        .await
                    {
                        Ok(_) => {
                            tx_clone.send(1).await.unwrap();
                        }
                        Err(e) => {
                            println!("[game_client_socket_clone] Error sending: {:?}", e);
                        }
                    };
                }
                Err(e) => println!("[game_server_socket_clone] Main error: {:?}", e),
            };
            buffer_pool_clone.return_buffer(buf).await;
        }
    });

    let watchdog = tokio::spawn(async move {
        loop {
            let recv = timeout(INACTIVITY_TIMEOUT, rx.recv());

            match recv.await {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(_) => {
                    println!("Client inactive for too long, removing mapping");
                    break;
                }
            }
        }
    });

    tokio::select! {
        _ = client_handler => {},
        _ = server_handler => {},
        _ = watchdog  => {},
    }

    Ok(())
}
