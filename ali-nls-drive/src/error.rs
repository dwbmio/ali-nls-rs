use std::env::VarError;

use crate::futures_channel::mpsc::TrySendError;
use thiserror::Error;
use tokio_tungstenite::tungstenite::Message;
#[derive(Error, Debug)]
pub enum ZError {
    #[error("resp struct fmt had been change, not found hash key {0}")]
    RespFmtError(String),

    #[error("sr result status failed!status={status:?}, msg={msg:?}")]
    StatusError { status: String, msg: String },

    #[error("ali-nls token get empty!set a config-file named *.env* in proj's root path")]
    AuthError(#[from] VarError),

    #[error(transparent)]
    WsConnectError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("sr result status failed!")]
    SenderError(#[from] TrySendError<Message>),


    #[error("io error! {0}")]
    IoError(#[from] std::io::Error),

    #[error("zerror! {0}")]
    CustomError(String),
}
