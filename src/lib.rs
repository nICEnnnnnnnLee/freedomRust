pub mod client;
pub mod server;
pub mod utils;

use std::net::ToSocketAddrs;

// use std::io;
use crate::client::config as client_config;
use crate::server::config as server_config;
use tokio::io;
use tokio::net::TcpListener;

pub async fn start_local(path: &String) -> io::Result<()> {
    // 读取并初始化配置
    client_config::init(path).unwrap();
    let conf = client_config::Config::global();
    println!("{:#?}", conf);
    let addr = (conf.bind_host.as_str(), conf.bind_port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;

    // 监听TCP连接
    let listener = TcpListener::bind(&addr).await?;
    loop {
        if let Ok((stream, _peer_addr)) = listener.accept().await {
            tokio::spawn(async move {
                if let Err(_err) = client::handle(stream, conf).await {
                    // eprintln!("{:?}", _err);
                }
            });
        }
    }
}

pub async fn start_remote(path: &String) -> io::Result<()> {
    // 读取并初始化配置
    server_config::init(path).unwrap();
    let conf = server_config::Config::global();
    println!("{:#?}", conf);
    let addr = (conf.bind_host.as_str(), conf.bind_port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;

    // 监听TCP连接
    let listener = TcpListener::bind(&addr).await?;
    if conf.use_ssl {
        let acceptor = server::init(conf)?;
        loop {
            match listener.accept().await {
                Ok((stream, _peer_addr)) => {
                    let acceptor = acceptor.clone();
                    match acceptor.accept(stream).await {
                        Ok(stream) => {
                            tokio::spawn(async move {
                                if let Err(_err) = server::handle(stream, conf).await {
                                    // eprintln!("TLS Handler err: {:?}", _err);
                                }
                            });
                        }
                        Err(_err) => {
                            eprintln!("Tls err: {:?}", _err);
                        }
                    }
                }
                Err(_err) => {
                    eprintln!("Tcp err: {:?}", _err);
                }
            }
        }
    } else {
        loop {
            match listener.accept().await {
                Ok((stream, _peer_addr)) => {
                    tokio::spawn(async move {
                        if let Err(_err) = server::handle(stream, conf).await {
                            // eprintln!("Normal Handler err: {:?}", _err);
                        }
                    });
                }
                Err(_err) => {
                    eprintln!("Tcp err: {:?}", _err);
                }
            }
        }
    }
}
