use ali_nls_drive::{
    self,
    error::ZError,
    futures_channel::{self, mpsc::UnboundedSender},
    tokio::{self, time::sleep},
    tokio_tungstenite::tungstenite::{http::Uri, Message},
    AliNlsDrive,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
    str::FromStr,
    time::Duration, future,
};
use uuid::Uuid;

pub use ali_nls_drive::config::AliNlsConfig;

pub struct AliNlsToSr {
    drive: AliNlsDrive,
    task_id: Option<String>,
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
    enable_words:bool
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
    TransEnd,
    Unknown,
}

impl AliNlsToSr {
    pub fn from(config: AliNlsConfig) -> Self {
        Self {
            drive: AliNlsDrive::new(config),
            task_id: None,
        }
    }

    fn gen_taskid() -> String {
        return Uuid::new_v4().to_string().replace("-", "");
    }

    fn get_token(&self) -> String {
        return "a5c722575c434f36a8d5879c46ff4fdc".to_owned();
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
                        return TransStep::TransEnd;
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
        let cmd = Self::gen_req_val(
            Self::gen_taskid(),
            self.drive.config.app_key.to_owned(),
            "StartTranscription".to_owned(),
        );
        let cont = json!(cmd).to_string();
        let _ = &ch_sender.unbounded_send(Message::Text(cont))?;

        let mut r:String = String::from("");
        //listen response
        self.drive
            .run(ch_receive, |c, msg|  {
                println!("msg is -->>msg={:?}", msg);
                let ret: Value = serde_json::from_str(msg.unwrap().to_string().as_str())
                    .expect("[ws]return msg convert to json failed!");
                let s = Self::handle_sr_resp(&ret);
                let _ = match s {
                    TransStep::UploadFile => {
                        let r = File::open(fpath.join("test").join("nls-sample-16k.wav"))
                            .expect("Not found test file!");
                        let sender_c = ch_sender.clone();
                        let _ = tokio::spawn(async move {
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
                        });
                    }
                    TransStep::Unknown => {}
                    TransStep::TransProcessing => {}
                    TransStep::TransEnd => {
                        r = ret.get("payload").unwrap().get("words").unwrap().to_string();
                        return future::ready(None);
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
                enable_inverse_text_normalization: true,
                enable_words: true
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
        let f = Path::new(cur_p);
        let r =
            File::open(f.join("test").join("nls-sample-16k.wav")).expect("Not found test file!");
        let ret = c.sr_from_slicefile(f).await;
        println!("runtime end");
        match ret {
            Ok(r) => if let Some(r_) = r {
                println!("json result is :{:?}", r_);
            },
            Err(e) => {
                println!("[error]{}", e.to_string());
            }
        }
    });
    println!("runtime end2");
}
