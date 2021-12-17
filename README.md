# tcp-proxy
A TCP Proxy (Tcp - Tcp, Unix - Unix, Tcp - Unix, Unix - Tcp)

# static build
```
nano ~/.cargo/config

[build]
rustflags = ["-C", "target-feature=+crt-static"]
target = "x86_64-unknown-linux-gnu"

cargo build --release
```
