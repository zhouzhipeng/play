use std::io;
use std::net::SocketAddr;

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

    pub fn build(self) -> (Server, Receiver<Message>) {
        let (tx, rx) = async_channel::unbounded();
        let handler = MyHandler {
            data: vec![],
            tx,
        };
        let mut server = mailin_embedded::Server::new(handler);

        server
            .with_ssl(self.ssl_config)
            .expect("SslConfig error")
            .with_addr(self.socket.unwrap())
            .unwrap();

        if self.auth {
            server.with_auth(AuthMechanism::Plain);
        }

        info!("listening on smtp://{}", self.socket.unwrap());

        (Server(server), rx)
    }
}

#[derive(Clone)]
pub struct MyHandler {
    pub data: Vec<u8>,
    tx: Sender<Message>
}

impl Handler for MyHandler {
    fn data(&mut self, buf: &[u8]) -> io::Result<()> {
        self.data.append(&mut buf.to_owned());

        Ok(())
    }

    fn data_end(&mut self) -> Response {
        let message = Message::from(&self.data).unwrap();

        info!("email in >> {}", message.subject);
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
