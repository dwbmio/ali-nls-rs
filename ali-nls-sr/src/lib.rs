use ali_nls_drive::{
    self,
    error::ZError,
    futures_channel::{self},
    tokio::{self, time::sleep},
    tokio_tungstenite::tungstenite::{http::Uri, Message},
    AliNlsDrive,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs::File,
    future,
    io::{BufReader, Read},
    path::Path,
    str::FromStr,
    sync::Arc,
    time::Duration,
};
use uuid::Uuid;

pub use ali_nls_drive::config::AliNlsConfig;

pub struct AliNlsToSr {
    drive: AliNlsDrive,
}

#[derive(Serialize)]
struct NlsHeader {
    message_id: String,
    task_id: String,
    namespace: String,
    name: String,
    appkey: String,
}

#[derive(Serialize)]
struct Payload {
    fomrat: String,
    sample_rate: u32,
    enable_intermediate_result: bool,
    enable_punctuation_prediction: bool,
    enable_inverse_text_normalization: bool,
    enable_words: bool,
}

#[derive(Serialize)]
struct CmdCont {
    header: NlsHeader,
    payload: Payload,
}
#[derive(PartialEq)]
enum TransStep {
    UploadFile,
    TransProcessing,
    TransOneSentenceEnd,
    TransAllComplete,
    Unknown,
}

impl AliNlsToSr {
    pub fn from(config: AliNlsConfig) -> Self {
        Self {
            drive: AliNlsDrive::new(config),
        }
    }

    fn gen_taskid() -> String {
        return Uuid::new_v4().to_string().replace("-", "");
    }

    fn get_token(&self) -> String {
        return "2cf7e5601a07495a8e56edadec2b0e0b".to_owned();
    }

    fn handle_sr_resp(ret: &Value) -> TransStep {
        let header = ret["header"].as_object().unwrap();
        if let Some(statu) = header.get("status") {
            let s = statu.as_i64().unwrap();
            if s == 20000000 {
                let proce_name = header.get("name").unwrap().as_str().unwrap();
                let _ = match proce_name {
                    "TranscriptionResultChanged" => {
                        return TransStep::TransProcessing;
                    }
                    "TranscriptionStarted" => {
                        return TransStep::UploadFile;
                    }
                    "SentenceEnd" => {
                        return TransStep::TransOneSentenceEnd;
                    }
                    "TranscriptionCompleted" => {
                        return TransStep::TransAllComplete;
                    }
                    &_ => {}
                };
                return TransStep::Unknown;
            }
        }
        return TransStep::Unknown;
    }

    pub async fn sr_from_slicefile(&mut self, fpath: &Path) -> Result<Option<String>, ZError> {
        let (ch_sender, ch_receive) = futures_channel::mpsc::unbounded();
        //url
        let sr_path = format!("/ws/v1?token={}", self.get_token());
        let _ = &self.drive.config.host.push_str(&sr_path);
        let uri = Uri::from_str(&self.drive.config.host).unwrap();
        //client
        self.drive.new_wscli(uri.to_string()).await;
        //shake params
        let task_id = Arc::new(Self::gen_taskid().clone());
        let app_key = Arc::new(self.drive.config.app_key.clone());
        let cmd = Self::gen_req_val(
            task_id.as_ref().to_string(),
            app_key.as_ref().to_string(),
            "StartTranscription".to_owned(),
        );
        let cont = json!(cmd).to_string();
        let _ = &ch_sender.unbounded_send(Message::Text(cont))?;

        let mut r: String = String::from("");
        //listen response
        self.drive
            .run(ch_receive, |_c, msg| {
                println!("msg is -->>msg={:?}", msg);
                let ret: Value = serde_json::from_str(msg.unwrap().to_string().as_str())
                    .expect("[ws]return msg convert to json failed!");
                let s = Self::handle_sr_resp(&ret);
                let _ = match s {
                    TransStep::UploadFile => {
                        //chk file open succ?
                        let r = File::open(fpath).expect("Not found test file!");
                        //clone outer var
                        let sender_c = ch_sender.clone();
                        let task_idr = task_id.as_ref().clone();
                        let app_keyr = app_key.as_ref().clone();
                        //slice upload 
                        let _: tokio::task::JoinHandle<()> = tokio::spawn(async move {
                            let mut reader = BufReader::new(r);
                            const CHUNK_SIZE: usize = 1024 * 10;
                            let mut chunk_con = [0_u8; CHUNK_SIZE];
                            loop {
                                let chunk: &mut [u8] = &mut chunk_con;
                                if reader.read_exact(chunk).is_ok() {
                                    println!("send file slice:{}", CHUNK_SIZE.to_string());
                                    let _ = &sender_c
                                        .unbounded_send(Message::Binary(chunk.to_vec()))
                                        .unwrap();
                                    sleep(Duration::from_millis(100)).await;
                                } else {
                                    println!("slice upload finish!");
                                    break;
                                }
                            }
                            let cmd = Self::gen_req_val(
                                task_idr,
                                app_keyr,
                                "StopTranscription".to_owned(),
                            );
                            let cont = json!(cmd).to_string();
                            let _ = &sender_c.unbounded_send(Message::Text(cont));
                        });
                    }
                    TransStep::Unknown => {}
                    TransStep::TransProcessing => {}
                    TransStep::TransOneSentenceEnd => {
                        r = ret.get("payload").unwrap().to_string();
                    }
                    TransStep::TransAllComplete => {
                        return future::ready(None)
                    }
                };
                future::ready(Some("".to_string()))
            })
            .await;
        Ok(Some(r))
    }

    fn gen_req_val(task_id: String, app_key: String, cmd: String) -> CmdCont {
        CmdCont {
            header: NlsHeader {
                message_id: Uuid::new_v4().to_string().replace("-", ""),
                task_id: task_id,
                namespace: "SpeechTranscriber".to_owned(),
                name: cmd,
                appkey: app_key,
            },
            payload: Payload {
                fomrat: "opus".to_owned(),
                sample_rate: 16000,
                enable_intermediate_result: false,
                enable_punctuation_prediction: true,
                enable_inverse_text_normalization: false,
                enable_words: true,
            },
        }
    }
}

#[test]
fn test_sr() {
    use std::env;
    use std::path::Path;
    use tokio::runtime::Runtime;

    Runtime::new().unwrap().block_on(async {
        let mut c = AliNlsToSr::from(AliNlsConfig {
            app_key: "FPwxKxga3cQ6B2Fs".to_owned(),
            host: "wss://nls-gateway.cn-shanghai.aliyuncs.com".to_owned(),
        });
        let cur_p = &env::current_dir().unwrap();
        let f = Path::new(cur_p).join("test").join("16000_2_s16le.wav");
        let ret = c.sr_from_slicefile(f.as_path()).await;
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
