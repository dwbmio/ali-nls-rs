use std::fmt;

///
/// zspeech配置：
/// * 注册todo
/// >email:xx@xx.xx


#[derive(Debug, Default)]
pub struct AliNlsConfig {
    pub app_key: String,
    pub host: String,
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
