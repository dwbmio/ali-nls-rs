use std::{future, io::Write, path::Path, str::FromStr, sync::Arc};

use ali_nls_drive::{
    error::ZError,
    futures_channel,
    tokio_tungstenite::tungstenite::{http::Uri, Message},
    AliNlsDrive, config::{CmdCont, NlsHeader},
};
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use ali_nls_drive::config::AliNlsConfig;
pub struct AliNlsSs {
    drive: AliNlsDrive,
}

#[derive(PartialEq, Debug)]
enum SyhesisStep {
    TransProcessing,
    SyhthesisComplete,
    Unknown,
}

#[derive(Serialize)]
struct Payload {
    text: String,
    format: String
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

    pub async fn ss_to_audio(
        &mut self,
        fpath: &Path,
        ss_txt: &str,
    ) -> Result<Option<String>, ZError> {
        let (ch_sender, ch_receive) = futures_channel::mpsc::unbounded();
        //chk write file exist
        let par = fpath
            .parent()
            .expect(format!("{}'s parent path get failed!", fpath.display().to_string()).as_str());
        if !par.is_dir() {
            std::fs::create_dir_all(par)?;
        }
        if !fpath.is_file() {
            ZError::CustomError(format!(
                "Write to {} failed!Already exists! Reste a path or use `force` mode",
                fpath.display()
            ));
        }
        // std::fs::
        let mut f = std::fs::File::create(fpath)?;
        let uri = Uri::from_str(&self.drive.config.host).unwrap();
        //client
        //shake params
        let task_id = Arc::new(Self::gen_taskid().clone());
        let app_key = Arc::new(self.drive.config.app_key.clone());
        let cmd = Self::gen_req_val::<Payload>(
            task_id.as_ref().to_string(),
            app_key.as_ref().to_string(),
            "StartSynthesis".to_owned(),
            ss_txt.to_owned()
        );
        let cont = json!(cmd).to_string();
        self.drive.new_wscli(uri.to_string()).await?;
        let _ = &ch_sender.unbounded_send(Message::Text(cont))?;
        //shakewave
        self.drive
            .run(ch_receive, |_c, msg| {
                let cont = &msg.unwrap().to_owned();
                let s = Self::handle_ss_resp(&cont);
                println!("match s is {:?}", s);
                let _ = match s {
                    SyhesisStep::TransProcessing => {
                        let e = f.write(&cont.to_owned().into_data());
                        println!("write...{:?}", e);
                    },
                    SyhesisStep::SyhthesisComplete => {
                        return future::ready(None);
                    },
                    SyhesisStep::Unknown => {
                        
                    },
                };
                future::ready(Some("".to_string()))
            })
            .await;
            drop(f);
        Ok(Some("".to_owned()))
    }

    fn handle_ss_resp(msg: &Message) -> SyhesisStep {
        if msg.is_text() {
            println!("msg is -->>msg={:?}", &msg);
            let ret: Value = serde_json::from_str(msg.to_string().as_str())
                .expect("[ws]return msg convert to json failed!");
            let header = ret["header"].as_object().unwrap();
            if let Some(statu) = header.get("status") {
                let s = statu.as_i64().unwrap();
                if s == 20000000 {
                    let proce_name = header.get("name").unwrap().as_str().unwrap();
                    let _ = match proce_name {
                        "SynthesisCompleted" => {
                            return SyhesisStep::SyhthesisComplete;
                        }
                        &_ => {}
                    };
                    return SyhesisStep::Unknown;
                }
            }
            return SyhesisStep::Unknown;
        } else if msg.is_binary() {
            return SyhesisStep::TransProcessing;
        }
        return SyhesisStep::Unknown;
    }

    fn gen_req_val<T>(task_id: String, app_key: String, cmd: String, text: String) -> CmdCont<Payload> {
        CmdCont {
            header: NlsHeader {
                message_id: Uuid::new_v4().to_string().replace("-", ""),
                task_id,
                namespace: "SpeechSynthesizer".to_owned(),
                name: cmd,
                appkey: app_key,
            },
            payload: Payload {
                text,
                format: "wav".to_owned()
            },
        }
    }
}

// wss://nls-gateway-cn-beijing.aliyuncs.com/ws/v1

#[test]
fn test_ss() {
    use std::env;
    use std::path::Path;
    use ali_nls_drive::tokio;
    use tokio::runtime::Runtime;

    Runtime::new().unwrap().block_on(async {
        let mut c = AliNlsSs::from(AliNlsConfig {
            app_key: "FPwxKxga3cQ6B2Fs".to_owned(),
            host: "wss://nls-gateway.aliyuncs.com/ws/v1".to_owned(),
        });
        let cur_p = &env::current_dir().unwrap();
        let f = Path::new(cur_p).join("test").join("out.wav");
        let ret = c.ss_to_audio(f.as_path(), "主要包括：neutral（中性）、happy（开心）、angry（生气）、sad（悲伤）、fear（害怕）、hate（憎恨）、surprise（惊讶）、arousal（激动）、serious（严肃）、disgust（厌恶）、jealousy（嫉妒）、embarrassed（尴尬）、frustrated（沮丧）、affectionate（深情）、gentle（温柔）、newscast（播报）、customer-service（客服）、story（小说）、living（直播）。").await;
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
