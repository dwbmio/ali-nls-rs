use ali_nls_drive::{
    self,
    error::ZError,
    futures_channel::{self, mpsc::SendError},
    gate::AliNlsDriveGate,
    tokio::{self},
    tokio_tungstenite::tungstenite::{http::Uri, Message},
    AliNlsDrive,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    env,
    fs::File,
    future,
    io::{BufReader, Read},
    str::FromStr,
};
use uuid::Uuid;

pub use ali_nls_drive::config::AliNlsConfig;

pub struct AliNlsToSr {
    drive: AliNlsDrive,
    token: Option<String>,
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
}

#[derive(Serialize)]
struct CmdCont {
    header: NlsHeader,
    payload: Payload,
}

#[derive(serde::Deserialize, Debug)]
pub struct WordSrResult {
    end: f32,
    start: f32,
    word: String,
}

impl AliNlsToSr {
    pub fn from(config: AliNlsConfig) -> Self {
        Self {
            drive: AliNlsDrive::new(config),
            token: None,
            task_id: None,
        }
    }

    fn get_token(&self) -> String {
        return "2784c2e219d24cd8a3bd804c56fd3c62".to_owned();
    }

    fn handle_sr_resp(ret: Value) -> Result<Vec<WordSrResult>, ZError> {
        let is_final = ret["is_final"].as_u64();
        let status = ret["status"].as_u64();
        if status.is_none() {
            return Err(ZError::RespFmtError("status".to_owned()));
        }
        let mut word_ret_l = vec![];
        if let Some(f) = is_final {
            if f == 0 {
                return Ok(word_ret_l);
            }
        }
        let result = ret["result"].as_object().unwrap();
        println!("handle_sr_resp result is {:?}", result);
        let result_l = result["result"].as_array();
        if result_l.is_none() {
            return Err(ZError::RespFmtError("result".to_owned()));
        }
        for wr in result_l.unwrap() {
            word_ret_l.push(serde_json::from_value(wr.to_owned()).unwrap());
        }
        Ok(word_ret_l)
    }

    pub async fn sr_from_slicefile(
        &mut self,
        r: File,
    ) -> Result<Option<Vec<WordSrResult>>, ZError> {
        let sr_path = format!("/ws/v1?token={}", self.get_token());
        let _ = &self.drive.config.host.push_str(&sr_path);
        let uri = Uri::from_str(&self.drive.config.host).unwrap();
        println!("ready ok!");

        let (ch_sender, ch_receive) = futures_channel::mpsc::unbounded();
        self.drive.new_wscli(uri.to_string()).await;
        tokio::spawn(Self::slice_upload(ch_sender, r));
        // tokio::spawn();
        // let _ = &self.hand_shake(ch_sender, "StartTranscription");
        // let  a= future::pending().await;
        // a.await;

        let ret = self
            .drive
            .run(ch_receive, |c, msg| {
                let data = msg.unwrap().into_data();
                let str_msg = String::from_utf8(data).unwrap();
                if str_msg.len() > 0 {
                    let ret: Value = serde_json::from_str(str_msg.as_str())
                        .expect("[ws]return msg convert to json failed!");
                    println!("resp msg =>>{:?}", ret);
                    let val = Self::handle_sr_resp(ret);
                    let l = val.unwrap();
                    if l.len() > 0 {
                        return future::ready(Some(l));
                    }
                }
                return future::ready(None);
            })
            .await;
        Ok(Some(vec![]))
    }

    async fn slice_upload(sender: futures_channel::mpsc::UnboundedSender<Message>, f: File) {
        //server gate
        // slice file
        let mut reader = BufReader::new(&f);
        const CHUNK_SIZE: usize = 1024 * 5;
        let mut chunk_con = [0_u8; CHUNK_SIZE];
        loop {
            let chunk: &mut [u8] = &mut chunk_con;
            if reader.read_exact(chunk).is_ok() {
                println!("send file slice:{}", CHUNK_SIZE.to_string());
                sender
                    .unbounded_send(Message::Binary(chunk.to_vec()))
                    .unwrap();
            } else {
                sender
                    .unbounded_send(Message::Text("EOS".to_owned()))
                    .unwrap();
                break;
            }
        }
        let c = future::pending();
        let () = c.await;
    }
    
    async fn hand_shake(&mut self, sender: futures_channel::mpsc::UnboundedSender<Message>, cmd: &str){
        self.cmd(sender, cmd);
    }

    fn gen_req_val(&mut self, cmd: String) -> CmdCont {
        if self.task_id.is_none() {
            self.task_id = Some(Uuid::new_v4().to_string().replace("-", ""));
        }
        CmdCont {
            header: NlsHeader {
                message_id: Uuid::new_v4().to_string().replace("-", ""),
                task_id: self.task_id.as_ref().unwrap().to_string(),
                namespace: "SpeechTranscriber".to_owned(),
                name: cmd,
                appkey: self.drive.config.app_key.clone(),
            },
            payload: Payload {
                fomrat: "opus".to_owned(),
                sample_rate: 16000,
                enable_intermediate_result: true,
                enable_punctuation_prediction: true,
                enable_inverse_text_normalization: true,
            },
        }
    }

    fn cmd(&mut self, sender: futures_channel::mpsc::UnboundedSender<Message>, cmd: &str) -> Result<(), futures_channel::mpsc::TrySendError<Message>> {
        //start 
        let f_cont = &self.gen_req_val(cmd.to_owned());
        let cont = json!(f_cont).to_string();
        let ret = sender.unbounded_send(Message::Text(cont));
        return ret;
        
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
        let r = File::open(f.join("test").join("P01_01.mp3")).expect("Not found test file!");
        let ret = c.sr_from_slicefile(r).await;
        println!("runtime end");
        match ret {
            Ok(r) => if let Some(r_) = r {},
            Err(e) => {
                println!("[error]{}", e.to_string());
            }
        }
        // if let Err(r) = ret {
        //     println!("[error]{}", r.to_string());
        // }
        // else
    });
    println!("runtime end2");
}
