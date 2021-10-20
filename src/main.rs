//! This example leverages `BytesCodec` to create a UDP client and server which
//! speak a custom protocol.
//!
//! Here we're using the codec from `tokio-codec` to convert a UDP socket to a stream of
//! client messages. These messages are then processed and returned back as a
//! new message with a new destination. Overall, we then use this to construct a
//! "ping pong" pair where two sockets are sending messages back and forth.

#![warn(rust_2018_idioms)]

use tokio::net::{ToSocketAddrs, UdpSocket};
use tokio::fs::File;
use tokio_stream;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;
use tokio_util::io::ReaderStream;

use futures::{SinkExt, StreamExt, TryStreamExt};
use std::env;
use std::error::Error;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let command = env::args()
        .nth(1)
        .unwrap_or_else(|| "both".to_string());

    let from_addr = env::args()
        .nth(2)
        .unwrap_or_else(|| "127.0.0.1:8000".to_string());
    
    let to_addr = env::args()
        .nth(3)
        .unwrap_or_else(|| "127.0.0.1:8001".to_string());

    match command.as_str() {
        "both" => both(from_addr, to_addr).await,
        "sink" => sink(from_addr, to_addr).await,
        "stream" => stream(to_addr).await,
        _ => panic!("wrong command")
    }
}

async fn both(from_addr: String, to_addr: String) -> Result<(), Box<dyn Error>> {
    let file = File::open("images/image.png").await?;
    let total = file.metadata().await?.len();
    let start = Instant::now();

    let a = udp_sink(from_addr, to_addr.parse()?);
    let b = udp_stream(to_addr);
    let (a, b) = tokio::join!(a, b);
    
    let elapsed = start.elapsed().as_secs_f64();
    let mb_per_sec = total as f64 / (elapsed - 5.0) / 1024.0 / 1024.0;
    dbg!(mb_per_sec);
    if a.is_err() {
        a
    } else {
        match b {
            Ok(_) => unreachable!("Should time out"),
            Err(_) => Ok(())
        }
    }
}

async fn sink(from_addr: String, to_addr: String) -> Result<(), Box<dyn Error>> {
    let file = File::open("images/image.png").await?;
    let total = file.metadata().await?.len();
    let start = Instant::now();

    let a = udp_sink(from_addr, to_addr.parse()?).await;

    let elapsed = start.elapsed().as_secs_f64();
    let mb_per_sec = total as f64 / elapsed / 1024.0 / 1024.0;
    dbg!(mb_per_sec);
    a
}

async fn stream(to_addr: String) -> Result<(), Box<dyn Error>> {
    let file = File::open("images/image.png").await?;
    let total = file.metadata().await?.len();
    let start = Instant::now();

    let b = dbg!(udp_stream(to_addr).await);
    
    let elapsed = start.elapsed().as_secs_f64();
    let mb_per_sec = total as f64 / (elapsed - 5.0) / 1024.0 / 1024.0;
    dbg!(mb_per_sec);
    match b {
        Ok(_) => unreachable!("Should time out"),
        Err(_) => Ok(())
    }
}

async fn udp_sink<A: ToSocketAddrs + std::fmt::Debug>(
    from_addr: A,
    to_addr: std::net::SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let mut socket = UdpFramed::new(UdpSocket::bind(from_addr).await?, BytesCodec::new());
    let mut reader_stream = ReaderStream::with_capacity(File::open("images/image.png").await?, 1024 * 9)
        .map(|r| r.map(|b| (b, to_addr)))
        .inspect(|r| {
            if let Ok(_b) = r {
                dbg!(_b.1);
            }
        })
        ;
    Ok(socket.send_all(&mut reader_stream).await?)
}

async fn udp_stream<A: ToSocketAddrs>(
    to_addr: A,
) -> Result<(), Box<dyn Error>> {
    let timeout = Duration::from_secs_f64(5.0);
    let socket = UdpFramed::new(UdpSocket::bind(to_addr).await?, BytesCodec::new())
        .inspect(|r| {
            if let Ok(_b) = r {
                dbg!(_b.0.len());
            }
        })
        .map(|e| e.unwrap().0);
    let socket = tokio_stream::StreamExt::timeout(socket, timeout);
    let socket = socket
        .try_fold((), |_, _| async move {Ok(())});
    Ok(socket.await?)
}