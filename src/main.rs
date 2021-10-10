//! This example leverages `BytesCodec` to create a UDP client and server which
//! speak a custom protocol.
//!
//! Here we're using the codec from `tokio-codec` to convert a UDP socket to a stream of
//! client messages. These messages are then processed and returned back as a
//! new message with a new destination. Overall, we then use this to construct a
//! "ping pong" pair where two sockets are sending messages back and forth.

#![warn(rust_2018_idioms)]

use tokio::net::UdpSocket;
use tokio::{io, time};
use tokio::fs::File;
// use tokio_stream::StreamExt;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;
use tokio_util::io::ReaderStream;

use bytes::Bytes;
use futures::{future, FutureExt, SinkExt, Stream, StreamExt, TryStreamExt};
use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:0".to_string());

    // Bind both our sockets and then figure out what ports we got.
    let a = UdpSocket::bind(&addr).await?;
    let b = UdpSocket::bind(&addr).await?;

    let b_addr = b.local_addr()?;

    let mut a = UdpFramed::new(a, BytesCodec::new());
    let b = UdpFramed::new(b, BytesCodec::new());

    let file = File::open("images/image.png").await?;
    let mut reader_stream = ReaderStream::with_capacity(file, 1024 * 5)
        .map(|b| b.map(|b| (b, b_addr)));

    let a = async move {
        a.send_all(&mut reader_stream).await.unwrap();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            a.send((Bytes::from("test"), b_addr)).await.unwrap();
        }
    };

    let b = b
        .map(|e| e.unwrap().0)
        .fold((0, Instant::now()), |(acc, start), e| async move {
            let total = acc + e.len();
            let mb_per_sec = total as f64 / start.elapsed().as_secs_f64() / 1024.0 / 1024.0;
            dbg!(mb_per_sec);
            (total, start)
        });

    let (a, b) = dbg!(tokio::join!(a, b));
    Ok(())
}

async fn recv(socket: &mut UdpFramed<BytesCodec>) -> Result<(), io::Error> {
    let mut total_bytes = 0;
    let start = std::time::Instant::now();
    while let Ok(Some(Ok((bytes, addr)))) = time::timeout(Duration::from_millis(200), socket.next()).await {
        total_bytes += bytes.len();
    }
    let elapsed = start.elapsed();
    dbg!(total_bytes, elapsed.as_secs_f64(), total_bytes as f64 / elapsed.as_secs_f64() / 1024.0 / 1024.0);
    Ok(())
}

async fn ping(socket: &mut UdpFramed<BytesCodec>, b_addr: SocketAddr) -> Result<(), io::Error> {
    socket.send((Bytes::from(&b"PING"[..]), b_addr)).await?;

    for _ in 0..4usize {
        let (bytes, addr) = socket.next().map(|e| e.unwrap()).await?;

        println!("[a] recv: {}", String::from_utf8_lossy(&bytes));

        socket.send((Bytes::from(&b"PING"[..]), addr)).await?;
    }

    Ok(())
}

async fn pong(socket: &mut UdpFramed<BytesCodec>) -> Result<(), io::Error> {
    let timeout = Duration::from_millis(200);

    while let Ok(Some(Ok((bytes, addr)))) = time::timeout(timeout, socket.next()).await {
        println!("[b] recv: {}", String::from_utf8_lossy(&bytes));

        socket.send((Bytes::from(&b"PONG"[..]), addr)).await?;
    }

    Ok(())
}