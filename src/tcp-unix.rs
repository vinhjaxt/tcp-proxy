//! A proxy that forwards data to another server and forwards that server's
//! responses back to clients.
//!
#![warn(rust_2018_idioms)]

use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream};
use tokio::net::{UnixListener, UnixStream};
use tokio::time::{sleep, Duration};
use std::net::{SocketAddr, ToSocketAddrs};

use futures::FutureExt;
use std::env;
use std::error::Error;
use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;

static LAST_DATA_DELAY: Duration = Duration::from_secs(1);

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let server_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    let listen_addr = env::args()
        .nth(2)
        .unwrap_or_else(|| "/tmp/unix.sock".to_string());

    println!("Listening on: {}", listen_addr);
    println!("Proxying to: {}", server_addr);

    let _ = fs::remove_file(&listen_addr).is_err();

    let listener = UnixListener::bind(&listen_addr)?;

    fs::set_permissions(listen_addr, Permissions::from_mode(0o777))?;

    while let Ok((inbound, _)) = listener.accept().await {
        let resolv_proxy_addrs = server_addr.clone().to_socket_addrs();
        if resolv_proxy_addrs.is_err() {
            eprintln!("Resolve error: invalid address");
            continue;
        }
        let resolv_proxy_addr = resolv_proxy_addrs.unwrap().next();
        if resolv_proxy_addr.is_none() {
            eprintln!("Resolve error no ip found");
            continue;
        }
        let transfer = transfer(inbound, resolv_proxy_addr.unwrap()).map(|r| {
            if let Err(e) = r {
                eprintln!("Failed to transfer; error={}", e);
            }
        });

        tokio::spawn(transfer);
    }

    Ok(())
}

async fn transfer(mut inbound: UnixStream, proxy_addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let mut outbound = match TcpStream::connect(proxy_addr).await {
        Err(e) => {
            let _ = inbound.shutdown();
            return Err(Box::new(e));
        }
        Ok(r) => r
    };

    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    tokio::select! {
        _ = async {
            let err = tokio::io::copy(&mut ri, &mut wo).await.err();
            sleep(LAST_DATA_DELAY).await;
            let _ = wo.shutdown().await;
            err
        } => {}
        _ = async {
            let err = tokio::io::copy(&mut ro, &mut wi).await.err();
            sleep(LAST_DATA_DELAY).await;
            let _ = wi.shutdown().await;
            err
        } => {}
    }; 

    let _ = wo.shutdown().await;
    let _ = wi.shutdown().await;

    Ok(())
}
