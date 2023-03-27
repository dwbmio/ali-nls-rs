### Overview

rust api for 

[ali-nls websocket doc](https://help.aliyun.com/document_detail/324262.html)

<mark>Attention:Implementation by test websocket lib, still in primary version</mark>

### Usage

```rust
let mut c = AliNlsToSr::from(AliNlsConfig {
  app_key: "$app_key".to_owned(),
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
```



### Test

#### sr
> cargo test --package ali-nls-sr --lib -- test_sr --exact --nocapture 
#### ss 
> cargo test --package ali-nls-ss --lib -- test_ss --exact --nocapture
