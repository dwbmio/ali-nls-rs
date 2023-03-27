use error::ZError;
pub use futures_channel;
pub use log;

use config::AliNlsConfig;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::{
    future, pin_mut,
    stream::{SplitSink, SplitStream},
    Future, StreamExt,
};
use std::{fmt::Debug, env::VarError};
use tokio::net::TcpStream;

pub use tokio;
pub use tokio_tungstenite;

use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message},
    MaybeTlsStream, WebSocketStream,
};

pub mod config;
pub mod error;
pub mod gate;

#[derive(Debug)]
pub struct AliNlsDrive {
    pub config: AliNlsConfig,
    writer: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    reader: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
}

impl AliNlsDrive {
    pub fn new(config: AliNlsConfig) -> AliNlsDrive {
        let _ = AliNlsDrive::load_env_file(".env_dev");
        Self {
            config,
            writer: None,
            reader: None,
        }
    }

    pub fn load_env_file(env_fname: &str) {
        let ret = dotenv::from_filename(env_fname);
        match ret {
            Ok(_) => {},
            Err(v) => {log::error!("found .env_dev file in {}! load env from it!", v.to_string())}
        };
    }

    pub async fn new_wscli(&mut self, mut full_uri: String) -> Result<(), ZError> {
        full_uri.push_str(format!("?token={}", &self.get_token()?).as_str());
        println!("connet uri is {}", full_uri);
        //connect
        let (ws_stream, _) = connect_async(full_uri).await?;
        println!("wss handshake has been succefully completed!");
        let (write, read) = ws_stream.split();
        self.writer = Some(write);
        self.reader = Some(read);
        Ok(())
    }

    fn get_token(&self) -> Result<String, VarError> {
        return std::env::var("ALI_TOKEN");
    }

    pub async fn run<F, T, Fut>(&mut self, receiver: UnboundedReceiver<Message>, handle: F)
    where
        T: Debug + Clone,
        F: FnMut(&mut Option<T>, Result<Message, tokio_tungstenite::tungstenite::Error>) -> Fut,
        Fut: Future<Output = Option<T>>,
        Self: Sized,
    {
        let task_sender = receiver
            .map(Ok)
            .forward(self.writer.as_mut().expect("new ws-client first!"));
        //read
        let def_ini: Option<T> = None;
        let task_reader = self
            .reader
            .as_mut()
            .expect("create client first!")
            .scan(def_ini, handle);
        pin_mut!(task_sender, task_reader);
        //wait once loop
        let _ = future::select(task_sender, task_reader.collect::<Vec<_>>()).await;
    }

    pub async fn close(&self) {}

    pub fn from_env() -> AliNlsDrive {
        let app_key = std::env::var("ZSPEECH_AKKEY").unwrap_or("UNSET_ZSPEECH_AKKEY".to_string());
        let host = std::env::var("ZSPEECH_HOST").unwrap_or("UNSET_ZSPEECH_HOST".to_string());
        Self {
            config: AliNlsConfig {
                app_key: app_key,
                host,
            },
            reader: None,
            writer: None,
        }
    }
}
