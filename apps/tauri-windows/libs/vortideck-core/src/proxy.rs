use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct Proxy {
    addr: SocketAddr,
}

impl Proxy {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub async fn start(&self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(self.addr).await?;
        println!("Proxy listening on {}", self.addr);

        loop {
            let (mut socket, _) = listener.accept().await?;
            tokio::spawn(async move {
                let mut buf = vec![0; 1024];
                loop {
                    match socket.read(&mut buf).await {
                        Ok(0) => break,
                        Ok(n) => {
                            if let Err(e) = socket.write_all(&buf[..n]).await {
                                eprintln!("Failed to write to socket: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to read from socket: {}", e);
                            break;
                        }
                    }
                }
            });
        }
    }
}
