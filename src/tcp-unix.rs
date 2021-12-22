use std::env;
use monoio::{
    io::{AsyncReadRent, AsyncWriteRentExt},
    net::{UnixListener, UnixStream, TcpStream},
};
use std::os::unix::fs::PermissionsExt;

#[monoio::main(entries = 512, timer_enabled = false)]
async fn main() {
    let target_address = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8081".to_string());
    let listen_addr = env::args()
        .nth(2)
        .unwrap_or_else(|| "/tmp/unix.sock".to_string());

    println!("Listening on: {}", listen_addr);

    let _ = std::fs::remove_file(&listen_addr).is_err();
    let listener = UnixListener::bind(&listen_addr)
        .unwrap_or_else(|_| panic!("unable to bind"));
    let _ = std::fs::set_permissions(listen_addr, std::fs::Permissions::from_mode(0o777)).is_err();

    println!("Proxying to: {}", target_address);
    loop {
        if let Ok((in_conn, _addr)) = listener.accept().await {
            let out_conn = TcpStream::connect(&target_address).await;
            if let Ok(out_conn) = out_conn {
                monoio::spawn(async move {
                    let _ = monoio::join!(
                      copy_one_direction_unix_tcp(&in_conn, &out_conn),
                      copy_one_direction_tcp_unix(&out_conn, &in_conn),
                    );
                    println!("relay finished");
                });
            } else {
                eprintln!("dial outbound connection failed");
            }
        } else {
            eprintln!("accept connection failed");
            return;
        }
    }
}

async fn copy_one_direction_tcp_unix(from: &TcpStream, to: &UnixStream) -> Result<Vec<u8>, std::io::Error> {
  let mut buf = Vec::with_capacity(8 * 1024);
  loop {
      // read
      let (res, _buf) = from.read(buf).await;
      buf = _buf;
      let res: usize = res?;
      if res == 0 {
          return Ok(buf);
      }

      // write all
      let (res, _buf) = to.write_all(buf).await;
      buf = _buf;
      res?;

      // clear
      buf.clear();
  }
}

async fn copy_one_direction_unix_tcp(from: &UnixStream, to: &TcpStream) -> Result<Vec<u8>, std::io::Error> {
  let mut buf = Vec::with_capacity(8 * 1024);
  loop {
      // read
      let (res, _buf) = from.read(buf).await;
      buf = _buf;
      let res: usize = res?;
      if res == 0 {
          return Ok(buf);
      }

      // write all
      let (res, _buf) = to.write_all(buf).await;
      buf = _buf;
      res?;

      // clear
      buf.clear();
  }
}
