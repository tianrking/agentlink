use crate::core::semantic::{CleanerConfig, StreamCleaner};
use anyhow::Result;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::mpsc;

pub async fn pump_with_semantic_channel<R, W>(
    mut reader: R,
    mut writer: W,
    cleaner_cfg: CleanerConfig,
) -> Result<()>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);

    let producer = tokio::spawn(async move {
        let mut buf = [0_u8; 4096];
        loop {
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            if tx.send(buf[..n].to_vec()).await.is_err() {
                break;
            }
        }
        Ok::<(), anyhow::Error>(())
    });

    let consumer = tokio::spawn(async move {
        let mut cleaner = StreamCleaner::new(cleaner_cfg);
        while let Some(chunk) = rx.recv().await {
            let processed = cleaner.push(&chunk);
            if !processed.is_empty() {
                writer.write_all(&processed).await?;
                writer.flush().await?;
            }
        }

        let tail = cleaner.flush();
        if !tail.is_empty() {
            writer.write_all(&tail).await?;
            if cleaner_cfg.enabled {
                writer.write_all(b"\n").await?;
            }
            writer.flush().await?;
        }

        Ok::<(), anyhow::Error>(())
    });

    producer.await??;
    consumer.await??;

    Ok(())
}
