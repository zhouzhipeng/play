use std::io;
use std::net::{IpAddr, SocketAddr};

use anyhow::Result;
use async_channel::{Receiver, Sender};
use log::info;
use mailin::AuthMechanism;
use mailin_embedded::{Handler, SslConfig};
use mailin_embedded::response::{self, Response};

use crate::models::message::Message;

pub struct Server(mailin_embedded::Server<MyHandler>);

impl Server {
    pub fn serve(self) -> Result<()> {
        self.0.serve().unwrap();

        Ok(())
    }
}
pub struct Builder {
    ssl_config: SslConfig,
    socket: Option<SocketAddr>,
    auth: bool,

}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {

        Builder {
            ssl_config: SslConfig::None,
            socket: None,
            auth: false,
        }
    }


    pub fn with_auth(mut self, value: bool) -> Self {
        self.auth = value;
        self
    }

    pub fn with_ssl(mut self, cert_path: Option<String>, key_path: Option<String>) -> Self {
        if let (Some(cert_path), Some(key_path)) = (cert_path, key_path) {
            self.ssl_config = SslConfig::SelfSigned {
                cert_path,
                key_path,
            };
        }
        self
    }


    pub fn bind(mut self, socket: SocketAddr) -> Self {
        self.socket = Some(socket);
        self
    }

    pub fn build(self, black_keywords: Vec<String>) -> (Server, Receiver<Message>) {
        let (tx, rx) = async_channel::unbounded();
        let handler = MyHandler {
            data: vec![],
            tx,
            black_keywords
        };
        let mut server = mailin_embedded::Server::new(handler);

        server
            .with_ssl(self.ssl_config)
            .expect("SslConfig error")
            .with_addr(self.socket.unwrap())
            .unwrap();

        // if self.auth {
        //     server.with_auth(AuthMechanism::Plain);
        // }

        info!("listening on smtp://{}", self.socket.unwrap());

        (Server(server), rx)
    }
}

#[derive(Clone)]
pub struct MyHandler {
    pub data: Vec<u8>,
    tx: Sender<Message>,
    black_keywords: Vec<String>
}

impl Handler for MyHandler {
    fn helo(&mut self, _ip: IpAddr, _domain: &str) -> Response {
        info!("email in helo >> ip : {:?}, domain : {:?}", _ip, _domain);

        for keyword in &self.black_keywords {
            if _ip.to_string().contains(keyword){
                return response::BLOCKED_IP
            }
            if _domain.contains(keyword){
                return response::BLOCKED_IP
            }
        }


        response::OK
    }

    fn mail(&mut self, _ip: IpAddr, _domain: &str, _from: &str) -> Response {
        info!("email in helo >> ip : {:?}, domain : {:?} , _from: {}", _ip, _domain, _from);
        for keyword in &self.black_keywords {
            if _from.contains(keyword){
                return response::BLOCKED_IP
            }
        }

        response::OK
    }

    fn data(&mut self, buf: &[u8]) -> io::Result<()> {
        self.data.append(&mut buf.to_owned());

        Ok(())
    }

    fn data_end(&mut self) -> Response {
        let message = Message::from(&self.data).unwrap();

        info!("email in >> subject : {}, sender : {}", message.subject, message.sender);

        for keyword in &self.black_keywords {
            if message.subject.contains(keyword){
                return response::BLOCKED_IP
            }
            if message.sender.contains(keyword){
                return response::BLOCKED_IP
            }
        }

        self.tx.send_blocking(message).unwrap();

        response::OK
    }

    fn auth_plain(
        &mut self,
        _authorization_id: &str,
        authentication_id: &str,
        password: &str,
    ) -> Response {
        response::AUTH_OK
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;
    use super::*;

    #[test]
    fn test_ip_addr_to_string() {
        let r = IpAddr::from_str("127.0.0.1").unwrap().to_string();
        assert_eq!("4a94d0f30@b013853be.jp".contains(".jp"), true);
        println!("ip : {}", r);
    }
}
