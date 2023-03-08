use std::env::VarError;

use thiserror::Error;
use tokio_tungstenite::tungstenite::Message;
use crate::futures_channel::mpsc::TrySendError;
#[derive(Error, Debug)]
pub enum ZError {
    
    #[error("resp struct fmt had been change, not found hash key {0}")]
    RespFmtError(String),

    #[error("sr result status failed!status={status:?}, msg={msg:?}")]
    StatusError{
        status: String,
        msg: String
    },
    
    #[error("ali-nls token get empty!set a config-file named *.env* in proj's root path")]
    AuthError(#[from] VarError),

    #[error("sr result status failed!")]
    SenderError(#[from] TrySendError<Message>)
}
