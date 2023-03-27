use std::{path::Path, sync::Arc, future, str::FromStr};

use ali_nls_drive::{tokio, AliNlsDrive, futures_channel, tokio_tungstenite::tungstenite::{http::Uri, Message}, error::ZError};
use log::info;
use serde::Serialize;
use serde_json::{Value, json};
use uuid::Uuid;

use ali_nls_drive::config::AliNlsConfig;
pub struct AliNlsSs {
    drive: AliNlsDrive,
}


#[derive(Serialize)]
struct NlsHeader {
    appkey: String,
    text: String
}

#[derive(Serialize)]
struct CmdCont {
    header: NlsHeader,
    payload: Payload,
}
// #[derive(Serialize)]
// struct NlsHeader {
//     message_id: String,
//     task_id: String,
//     namespace: String,
//     name: String,
//     appkey: String,
// }



#[derive(Serialize)]
struct Payload {
    text:String
}

impl AliNlsSs {
    pub fn from(config: AliNlsConfig) -> Self {
        Self {
            drive: AliNlsDrive::new(config),
        } 
    }

    fn gen_taskid() -> String {
        return Uuid::new_v4().to_string().replace("-", "");
    }

    pub async fn ss_to_audio(&mut self, fpath: &Path, ss_txt: &str) -> Result<Option<String>, ZError>{
        let (ch_sender, ch_receive) = futures_channel::mpsc::unbounded();
        let uri = Uri::from_str(&self.drive.config.host).unwrap();
        //client
        //shake params
        let task_id = Arc::new(Self::gen_taskid().clone());
        let app_key = Arc::new(self.drive.config.app_key.clone());
        let cmd = Self::gen_req_val(
            task_id.as_ref().to_string(),
            app_key.as_ref().to_string(),
            "StartSynthesis".to_owned(),
        );
        let cont = json!(cmd).to_string();
        self.drive.new_wscli(uri.to_string()).await?;
        &ch_sender.unbounded_send(Message::Text(cont))?;

        self.drive
            .run(ch_receive, |_c, msg| {
                println!("msg is -->>msg={:?}", msg);
                future::ready(Some("".to_string()))
            })
            .await;
        Ok(Some("".to_owned()))

    }

    fn gen_req_val(task_id: String, app_key: String, cmd: String) -> CmdCont {
        CmdCont {
            header: NlsHeader {
                appkey: app_key,
                text: "rwr".to_owned(),
            },
            payload: Payload {
                text: "tsetsts".to_owned(),
            }
        }
    }

    
}


// wss://nls-gateway-cn-beijing.aliyuncs.com/ws/v1

#[test]
fn test_ss() {
    use std::env;
    use std::path::Path;
    use tokio::runtime::Runtime;

    Runtime::new().unwrap().block_on(async {
        let mut c = AliNlsSs::from(AliNlsConfig {
            app_key: "FPwxKxga3cQ6B2Fs".to_owned(),
            host: "wss://nls-gateway.aliyuncs.com".to_owned(),
        });
        let cur_p = &env::current_dir().unwrap();
        let f = Path::new(cur_p).join("test").join("16000_2_s16le.wav");
        let ret = c.ss_to_audio(f.as_path(),"131231").await;
        match ret {
            Ok(r) => {
                if let Some(r_) = r {
                    println!("json result is :{:?}", r_);
                }
            }
            Err(e) => {
                println!("[error]{}", e.to_string());
            }
        }
    });
}
