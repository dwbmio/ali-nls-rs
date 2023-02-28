use thiserror::Error;
use tokio_tungstenite::tungstenite::Message;
use crate::futures_channel::mpsc::TrySendError;
#[derive(Error, Debug)]
pub enum ZError {
    
    #[error("[Drive]resp struct fmt had been change, not found hash key {0}")]
    RespFmtError(String),

    #[error("[Drive]sr result status failed!status={status:?}, msg={msg:?}")]
    StatusError{
        status: String,
        msg: String
    },

    #[error("[Drive]sr result status failed!")]
    SenderError(#[from] TrySendError<Message>)
    // #[error("send file slice failed!")]
    // SendError(#[from] WsError)
}

// impl Display for ZError {

// }