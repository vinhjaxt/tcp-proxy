#![warn(rust_2018_idioms)]

use tokio::io::AsyncWriteExt;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpStream, UnixListener, lookup_host};

use futures::FutureExt;
use std::env;
use std::error::Error;
use std::fs::{self, Permissions};
use std::os::unix::fs::PermissionsExt;

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

    while let Ok((mut inbound, _)) = listener.accept().await {
        let to_addr = server_addr.clone();
        tokio::spawn(async move {
            let addr = match lookup_host(to_addr).await {
                Err(e) => {
                    let _ = inbound.shutdown();
                    eprintln!("lookup: {}", e);
                    return;
                }
                Ok(mut r) => match r.next() {
                    None => {
                        let _ = inbound.shutdown();
                        eprintln!("lookup: no addr");
                        return;
                    }
                    Some(a) => a
                }
            };
    
            let mut outbound = match TcpStream::connect(addr).await {
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
