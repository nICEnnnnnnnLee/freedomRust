pub mod config;
use crypto::digest::Digest;
use crypto::md5::Md5;
use regex::Regex;
use rustls_pemfile::{certs, rsa_private_keys};
use std::fs::File;
use std::io::{self, BufReader};
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::time::SystemTime;
use tokio::io::{copy, split, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::rustls::{Certificate, PrivateKey};
use tokio_rustls::TlsAcceptor;

lazy_static::lazy_static! {
    static ref REG_COOKIE :Regex  = Regex::new(r"Cookie *: *([^\r\n]+)").unwrap();
    static ref REG_DOMAIN :Regex  = Regex::new(r"my_domain=([^;]+)").unwrap();
    static ref REG_PORT :Regex  = Regex::new(r"my_port=([0-9]+)").unwrap();
    static ref REG_TOKEN :Regex  = Regex::new(r"my_token=([^;]+)").unwrap();
    static ref REG_USERNAME :Regex  = Regex::new(r"my_username=([^;]+)").unwrap();
    static ref REG_TYPE :Regex  = Regex::new(r"my_type=([0-9]+)").unwrap();
    static ref REG_TIME :Regex  = Regex::new(r"my_time=([0-9]+)").unwrap();
    static ref RESPONSE_403:&'static str = concat!(
        "HTTP/1.1 403 Forbidden\r\n" ,"Content-Length: 0\r\n" ,"Connection: closed\r\n\r\n"
    );
    static ref RESPONSE_101:&'static str = concat!(
        "HTTP/1.1 101 Switching Protocols\r\n" ,
        "auth: ok\r\n" ,"Sec-WebSocket-Accept: YWJjZGVmZw==\r\n" ,
        "Upgrade: websocket\r\n" ,
        "Connection: Upgrade\r\n\r\n"
    );
}
pub fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

pub fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    rsa_private_keys(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
        .map(|mut keys| keys.drain(..).map(PrivateKey).collect())
}

pub fn init(conf: &config::Config) -> io::Result<TlsAcceptor> {
    let certs = load_certs(Path::new(&conf.cert_path))?;
    let mut keys = load_keys(Path::new(&conf.key_path))?;

    let server_conf = tokio_rustls::rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, keys.remove(0))
        .map_err(|_err| io::Error::new(io::ErrorKind::InvalidInput, "TLS cert loading error"))?;
    Ok(TlsAcceptor::from(std::sync::Arc::new(server_conf)))
}

// fn check_valid_and_get_dst_addr(head_str: &str, conf: &config::Config) -> io::Result<(&str, u16)> {
fn check_valid_and_get_dst_addr<'a>(
    head_str: &'a str,
    conf: &'a config::Config,
) -> io::Result<SocketAddr> {
    println!("request comming...");
    let cookie = REG_COOKIE
        .captures(head_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Cookie is not in header!!"))?
        .get(1)
        .map(|m| m.as_str())
        .unwrap();
    let domain = REG_DOMAIN
        .captures(cookie)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "domain is not in header!!"))?
        .get(1)
        .map(|m| m.as_str())
        .unwrap();
    let port = REG_PORT
        .captures(cookie)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "port is not in header!!"))?
        .get(1)
        .map(|m| m.as_str())
        .unwrap();
    let port = u16::from_str_radix(port, 10).unwrap();
    let token = REG_TOKEN
        .captures(cookie)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "token is not in header!!"))?
        .get(1)
        .map(|m| m.as_str())
        .unwrap();
    let username = REG_USERNAME
        .captures(cookie)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "username is not in header!!"))?
        .get(1)
        .map(|m| m.as_str())
        .unwrap();
    let username = &String::from(username);
    let proxy_type = REG_TYPE
        .captures(cookie)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "proxy_type is not in header!!"))?
        .get(1)
        .map(|m| m.as_str())
        .unwrap();
    let time_local_str = REG_TIME
        .captures(cookie)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "timestamp is not in header!!"))?
        .get(1)
        .map(|m| m.as_str())
        .unwrap();
    let time_local = u128::from_str_radix(time_local_str, 10).map_err(|_x| {
        io::Error::new(io::ErrorKind::Other, "Timestamp in header is not correct!!")
    })?;

    let time_now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    if (time_now > time_local + 600000) || proxy_type != "1" {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Timestamp or type in header is not valid!!",
        ));
    }
    match conf.users.get(username) {
        Some(pwd) => {
            let mut expected_token = Md5::new();
            expected_token.input_str(pwd);
            expected_token.input_str(&conf.salt);
            expected_token.input_str(time_local_str);
            let expected_token = expected_token.result_str();
            if expected_token != token {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "User Token not valid!!",
                ));
            }
        }
        None => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "No such user exists!!",
            ));
        }
    };
    println!("to {}:{}", domain, port);
    let addr = (domain, port)
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    Ok(addr)
}

pub async fn handle<IO>(stream: IO, conf: &config::Config) -> io::Result<()>
where
    IO: AsyncRead + AsyncWrite + Unpin + AsyncWriteExt,
{
    let (mut local_reader, mut local_writer) = split(stream);

    // 从头部读取信息
    let mut head = [0u8; 2048];

    let n = local_reader.read(&mut head[..]).await;
    if let Err(err) = n {
        let _ = local_writer.shutdown().await;
        return Err(err);
    }
    let n = n.unwrap();
    if n == 2048 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Receive a unexpected big size of header!!",
        ));
    }

    let head_str =
        std::str::from_utf8(&head[..n]).map_err(|x| io::Error::new(io::ErrorKind::Interrupted, x));
    if let Err(err) = head_str {
        let _ = local_writer.shutdown().await;
        return Err(err);
    }
    let head_str = head_str.unwrap();

    let addr = check_valid_and_get_dst_addr(head_str, conf);
    if let Err(err) = addr {
        eprintln!("{}", err);
        let _ = local_writer.write_all(RESPONSE_403.as_bytes()).await;
        let _ = local_writer.shutdown().await;
        return Err(err);
    }
    let addr = addr.unwrap();
    // 远程读取链接
    let dst_stream = TcpStream::connect(&addr).await;
    if let Err(err) = dst_stream {
        let _ = local_writer.shutdown().await;
        return Err(err);
    }
    let dst_stream = dst_stream.unwrap();

    let (mut dst_reader, mut dst_writer) = split(dst_stream);

    // 回复101.建立通道
    if let Err(err) = local_writer.write_all(RESPONSE_101.as_bytes()).await {
        let _ = local_writer.shutdown().await;
        return Err(err);
    }

    // let dst = format!("{}", addr);
    let client_to_dst = async {
        let _ = copy(&mut local_reader, &mut dst_writer).await;
        let _ = dst_writer.shutdown().await;
        // println!("remote {} 已关闭", dst);
        Ok(()) as io::Result<()>
    };
    let dst_to_client = async {
        let _ = copy(&mut dst_reader, &mut local_writer).await;
        let _ = local_writer.shutdown().await;
        // println!("local {} 已关闭", dst);
        Ok(())
    };

    let _ = tokio::try_join!(client_to_dst, dst_to_client);
    Ok(()) as io::Result<()>
}
