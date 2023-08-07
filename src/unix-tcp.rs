#![warn(rust_2018_idioms)]

use tokio::io::AsyncWriteExt;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, UnixStream};

use futures::FutureExt;
use std::env;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/unix.sock".to_string());
    let listen_addr = env::args()
        .nth(2)
        .unwrap_or_else(|| "127.0.0.1:8081".to_string());

    println!("Listening on: {}", listen_addr);
    println!("Proxying to: {}", server_addr);

    let listener = TcpListener::bind(listen_addr).await?;

    while let Ok((mut inbound, _)) = listener.accept().await {
        let to_addr = server_addr.clone();
        tokio::spawn(async move {    
            let mut outbound = match UnixStream::connect(to_addr).await {
                Err(e) => {
                    let _ = inbound.shutdown();
                    eprintln!("connect: {}", e);
                    return;
                }
                Ok(r) => r
            };

            copy_bidirectional(&mut inbound, &mut outbound).map(|r| {
                if let Err(e) = r {
                    eprintln!("transfer: {}", e);
                }
            })
            .await
        });
    }
    Ok(())
}
