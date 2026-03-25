use bitcode::{Decode, Encode};
use iroh::endpoint::{RecvStream, SendStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Maximum message size: 16 MiB. Prevents OOM from malicious/buggy peers.
pub const MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

pub async fn write_p2p<T>(mut send_stream: SendStream, message: T) -> anyhow::Result<()>
where
    T: Encode + std::fmt::Debug,
{
    let encoded = bitcode::encode(&message);

    send_stream.write_u32(encoded.len() as u32).await?;
    send_stream.write_all(&encoded).await?;

    send_stream.finish()?;
    Ok(())
}

pub async fn read_p2p<T>(mut recv_stream: RecvStream) -> anyhow::Result<T>
where
    T: for<'a> Decode<'a>,
{
    let size = recv_stream.read_u32().await?;

    anyhow::ensure!(
        size <= MAX_MESSAGE_SIZE,
        "message size {size} exceeds maximum {MAX_MESSAGE_SIZE}"
    );

    let mut buff = vec![0u8; size as usize];
    recv_stream.read_exact(&mut buff).await?;

    let message: T = bitcode::decode(&buff)?;

    Ok(message)
}
