pub mod config;

use crate::utils::tls;
use crypto::digest::Digest;
use crypto::md5::Md5;
use regex::Regex;
use std::time::SystemTime;
use tokio::io::{self, copy, split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;

lazy_static::lazy_static! {
    static ref REG_HEAD :Regex  = Regex::new(r"(CONNECT|Host:) ([^ :\r\n]+)(?::(\d+))?").unwrap();
}

fn get_auth_header(conf: &config::Config, domain: &str, port: &str) -> String {
    let time_now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let mut token = Md5::new();
    token.input_str(&conf.password);
    token.input_str(&conf.salt);
    token.input_str(&time_now.to_string());
    let token = token.result_str();
    let cookies = format!(
        "my_type=1; my_domain={}; my_port={}; my_username={}; my_time={}; my_token={}",
        domain, port, conf.username, time_now, token
    );
    format!(
        concat!(
            "GET {} HTTP/1.1\r\n",
            "Host: {}:{}\r\n",
            "User-Agent: {}\r\n",
            "Accept: */*\r\n",
            "Accept-Language: zh-CN,zh;q=0.8,zh-TW;q=0.7,zh-HK;q=0.5,en-US;q=0.3,en;q=0.2\r\n",
            "Sec-WebSocket-Version: 13\r\n",
            "Sec-WebSocket-Extensions: permessage-deflate\r\n",
            "Sec-WebSocket-Key: YWJjZGVmZw==\r\n",
            "Connection: keep-alive, Upgrade\r\n",
            "Pragma: no-cache\r\n",
            "Cache-Control: no-cache\r\n",
            "Upgrade: websocket\r\n",
            "Cookie: {}\r\n\r\n"
        ),
        conf.http_path, conf.http_domain, conf.remote_port, conf.http_user_agent, cookies
    )
}

async fn get_remote_conn(
    domain: &str,
    port: &str,
    conf: &config::Config,
) -> io::Result<(
    ReadHalf<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>,
    WriteHalf<tokio_rustls::client::TlsStream<tokio::net::TcpStream>>,
)> {
    // let conf = Config::global();
    let header = get_auth_header(conf, domain, port);
    let (mut reader, mut writer) = tls::connect(
        &conf.remote_host,
        conf.remote_port,
        &conf.http_domain,
        conf.allow_insecure,
    )
    .await?;
    writer.write_all(header.as_bytes()).await?;

    let mut head = [0u8; 2048];
    let n = reader.read(&mut head[..]).await?;

    if n == 2048 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Receive a unexpected big size of header!!",
        ));
    }
    let head_str = std::str::from_utf8(&head[..n])
        .map_err(|x| io::Error::new(io::ErrorKind::Interrupted, x))?;

    if head_str.contains("auth: ok") {
        Ok((reader, writer))
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Auth is not right!!"))
    }
}

pub async fn handle(stream: TcpStream, conf: &config::Config) -> io::Result<()> {
    let (mut local_reader, mut local_writer) = split(stream);
    // 读取头部
    let mut head = [0u8; 2048];
    let n = local_reader.read(&mut head[..]).await;
    if let Err(err) = n {
        let _ = local_writer.shutdown().await;
        return Err(err);
    }
    let n = n.unwrap();

    let head_str =
        std::str::from_utf8(&head[..n]).map_err(|x| io::Error::new(io::ErrorKind::Interrupted, x));
    if let Err(err) = head_str {
        let _ = local_writer.shutdown().await;
        return Err(err);
    }
    let head_str = head_str.unwrap();

    if let Some(caps) = REG_HEAD.captures(head_str) {
        let host = &caps[2];
        let port = caps.get(3).map_or("80", |m| m.as_str());
        // println!("{} {}", host, port);
        // 以下是直连
        // let dst_addr = format!("{}:{}", host, port);
        // let remote_stream = TcpStream::connect(dst_addr).await?;
        // let (mut remote_reader, mut remote_writer) = split(remote_stream);

        let result = get_remote_conn(host, port, conf).await;
        if let Err(err) = result {
            let _ = local_writer.shutdown().await;
            return Err(err);
        }
        let (mut remote_reader, mut remote_writer) = result.unwrap();

        if head_str.starts_with("CONNECT") {
            if let Err(err) = local_writer
                .write_all("HTTP/1.1 200 Connection Established\r\n\r\n".as_bytes())
                .await
            {
                let _ = local_writer.shutdown().await;
                return Err(err);
            }
        } else {
            if let Err(err) = remote_writer.write_all(&head[..n]).await {
                let _ = local_writer.shutdown().await;
                return Err(err);
            }
        }

        // let dst = format!("{}:{}", host, port);
        let client_to_server = async {
            let _ = copy(&mut local_reader, &mut remote_writer).await;
            let _ = remote_writer.shutdown().await;
            // println!("remote {} 已关闭", dst);
            Ok(()) as io::Result<()>
        };

        let server_to_client = async {
            let _ = copy(&mut remote_reader, &mut local_writer).await;
            let _ = local_writer.shutdown().await;
            // println!("local {} 已关闭", dst);
            Ok(())
        };

        let _ = tokio::try_join!(client_to_server, server_to_client);
        Ok(())
    } else {
        let _ = local_writer.shutdown().await;
        Err(io::Error::new(io::ErrorKind::Other, "Not valid head"))
    }
}
