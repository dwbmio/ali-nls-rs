use crypto::{
    aes, blockmodes,
    buffer::{self, BufferResult, ReadBuffer, WriteBuffer},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AliNlsDriveGate {
    pub fr: String,
    pub uid: String,
    pub domain: String,
    pub cuid: String,
    pub ak: String,
}

impl AliNlsDriveGate {
    pub fn aes_cbc_cliper(data: &[u8], key: &[u8], iv: &[u8]) -> Option<Vec<u8>> {
        let mut encryptor =
            aes::cbc_encryptor(aes::KeySize::KeySize256, key, iv, blockmodes::PkcsPadding);
        let mut final_result = Vec::<u8>::new();
        let mut read_buffer = buffer::RefReadBuffer::new(data);
        let mut buffer = [0; 4096];
        let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

        loop {
            let result = encryptor.encrypt(&mut read_buffer, &mut write_buffer, true);

            // "write_buffer.take_read_buffer().take_remaining()" means:
            // from the writable buffer, create a new readable buffer which
            // contains all data that has been written, and then access all
            // of that data as a slice.
            final_result.extend(
                write_buffer
                    .take_read_buffer()
                    .take_remaining()
                    .iter()
                    .map(|&i| i),
            );

            match result {
                Ok(BufferResult::BufferUnderflow) => break,
                Ok(BufferResult::BufferOverflow) => {}
                Err(_) => todo!(),
            }
        }
        Some(final_result)
    }
    
}