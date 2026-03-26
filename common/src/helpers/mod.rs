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

/// Write a single frame without closing the stream. Used for streaming responses
/// where multiple frames are sent before finish().
pub async fn write_frame<T>(send_stream: &mut SendStream, message: T) -> anyhow::Result<()>
where
    T: Encode + std::fmt::Debug,
{
    use tokio::io::AsyncWriteExt as _;

    let encoded = bitcode::encode(&message);

    send_stream.write_u32(encoded.len() as u32).await?;
    send_stream.write_all(&encoded).await?;
    send_stream.flush().await?;

    Ok(())
}

/// Read a single frame without expecting the stream to close.
/// Returns None if the stream has been finished by the sender.
pub async fn read_frame<T>(recv_stream: &mut RecvStream) -> anyhow::Result<Option<T>>
where
    T: for<'a> Decode<'a>,
{
    let size = match recv_stream.read_u32().await {
        Ok(s) => s,
        Err(_) => return Ok(None), // Stream finished
    };

    anyhow::ensure!(
        size <= MAX_MESSAGE_SIZE,
        "message size {size} exceeds maximum {MAX_MESSAGE_SIZE}"
    );

    let mut buff = vec![0u8; size as usize];
    recv_stream.read_exact(&mut buff).await?;

    let message: T = bitcode::decode(&buff)?;

    Ok(Some(message))
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
