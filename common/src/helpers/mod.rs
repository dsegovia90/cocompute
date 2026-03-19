use bitcode::{Decode, Encode};
use iroh::endpoint::{RecvStream, SendStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn write_p2p<T>(mut send_stream: SendStream, message: T) -> anyhow::Result<()>
where
    T: Encode + std::fmt::Debug,
{
    let encoded = bitcode::encode(&message);

    send_stream.write_u32(encoded.len() as u32).await?;
    send_stream.write_all(&encoded).await?;

    send_stream.finish()?;
    send_stream.stopped().await?;
    Ok(())
}

pub async fn read_p2p<T>(mut recv_stream: RecvStream) -> anyhow::Result<T>
where
    T: for<'a> Decode<'a>,
{
    let size = recv_stream.read_u32().await?;

    let mut buff = vec![0u8; size as usize];
    recv_stream.read_exact(&mut buff).await?;

    let message: T = bitcode::decode(&buff)?;

    Ok(message)
}
