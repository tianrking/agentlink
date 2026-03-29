use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;

#[derive(Debug, Serialize, Deserialize)]
pub enum Event {
    ExitCode { code: Option<i32> },
    Error { message: String },
}

pub struct StatusEmitter {
    socket_path: Option<PathBuf>,
}

impl StatusEmitter {
    pub fn new(socket_path: Option<PathBuf>) -> Self {
        Self { socket_path }
    }

    pub async fn emit(&mut self, event: Event) {
        let Some(path) = &self.socket_path else {
            return;
        };

        let Ok(mut stream) = UnixStream::connect(path).await else {
            return;
        };

        let Ok(payload) = postcard::to_allocvec(&event) else {
            return;
        };

        let len = (payload.len() as u32).to_be_bytes();
        let _ = stream.write_all(&len).await;
        let _ = stream.write_all(&payload).await;
        let _ = stream.flush().await;
    }
}
