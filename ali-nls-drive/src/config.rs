use std::fmt;

use serde::Serialize;
///
/// zspeech配置：
/// * 注册todo
/// >email:xx@xx.xx

#[derive(Debug, Default)]
pub struct AliNlsConfig {
    pub app_key: String,
    pub host: String,
}

#[derive(Serialize)]
pub struct NlsHeader {
    pub message_id: String,
    pub task_id: String,
    pub namespace: String,
    pub name: String,
    pub appkey: String,
}

#[derive(Serialize)]
pub struct CmdCont<T> {
    pub header: NlsHeader,
    pub payload: T,
}

impl fmt::Display for AliNlsConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ZSpeech config=> \n{{\n  host = {}\n  app_key = {}}}",
            self.host, self.app_key
        )
    }
}
