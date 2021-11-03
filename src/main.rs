//! This example leverages `BytesCodec` to create a UDP client and server which
//! speak a custom protocol.
//!
//! Here we're using the codec from `tokio-codec` to convert a UDP socket to a stream of
//! client messages. These messages are then processed and returned back as a
//! new message with a new destination. Overall, we then use this to construct a
//! "ping pong" pair where two sockets are sending messages back and forth.

#![warn(rust_2018_idioms)]

use std::env;
use std::error::Error;
use std::time::{Duration, Instant};

use bytes::BytesMut;
use tokio::net::{ToSocketAddrs, UdpSocket};
use tokio::fs::File;
use tokio_stream;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;
use tokio_util::io::ReaderStream;
use futures::{SinkExt, StreamExt, TryStreamExt, future};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();
    let command = env::args()
        .nth(1)
        .unwrap_or_else(|| "both".to_string());

    let from_addr = env::args()
        .nth(2)
        .unwrap_or_else(|| "0.0.0.0:8000".to_string());
    
    let to_addr = env::args()
        .nth(3)
        .unwrap_or_else(|| "0.0.0.0:8001".to_string());

    let throttle = Duration::from_secs_f64(0.05);
    let timeout = Duration::from_secs_f64(0.01);
    
    match command.as_str() {
        "both" => both(from_addr, to_addr, throttle, timeout).await,
        "sink" => udp_sink(from_addr, to_addr.parse()?, throttle).await,
        "stream" => udp_stream(to_addr, timeout).await,
        _ => panic!("wrong command")
    }
}

async fn both(from_addr: String, to_addr: String, throttle: Duration, timeout: Duration) -> Result<(), Box<dyn Error>> {
    let a = udp_sink(from_addr, to_addr.parse()?, throttle);
    let b = udp_stream(to_addr, timeout);
    let (a, b) = tokio::join!(a, b);
    a.and(b)
}

async fn udp_sink<A: ToSocketAddrs + std::fmt::Debug + Clone>(
    from_addr: A,
    to_addr: std::net::SocketAddr,
    throttle: Duration,
) -> Result<(), Box<dyn Error>> {
    loop {
        let mut socket = UdpFramed::new(UdpSocket::bind(from_addr.clone()).await?, BytesCodec::new());
        let reader_stream = ReaderStream::with_capacity(
            File::open("images/image.png").await?,
            1024 * 50
        )
            // .inspect(|r| {dbg!(r.as_deref().map(|b| b.len()));})
            .chunks(4)
            .scan((0, Instant::now()), |(acc, last_time), bytes| {
                let bytes: BytesMut = bytes.into_iter().map(Result::unwrap).flatten().collect();

                let time = Instant::now();
                let len = bytes.len();
                let speed = len as f64 / 1024.0 / 1024.0 / time.duration_since(*last_time).as_secs_f64();
                *last_time = time;
                *acc += len;
                println!("{:.0}\t{}\t{}", speed, *acc, len);
                future::ready(Some(bytes.freeze()))
            })
            .map(|b| Ok((b, to_addr)));
        tokio::pin!(reader_stream);
        let reader_stream = tokio_stream::StreamExt::throttle(
            reader_stream,
            throttle
        );
        tokio::pin!(reader_stream);
        socket.send_all(&mut reader_stream).await?;
    }
}

async fn udp_stream<A: ToSocketAddrs + Clone>(
    to_addr: A,
    timeout: Duration,
) -> Result<(), Box<dyn Error>> {
    let mut acc = 0;
    loop {
        let socket = UdpFramed::new(
            UdpSocket::bind(to_addr.clone()).await?,
            BytesCodec::new()
        )
            .map(|e| e.unwrap().0)
            .scan((0, Instant::now()), |(_, last_time), bytes| {
                let time = Instant::now();
                let len = bytes.len();
                let speed = len as f64 / 1024.0 / 1024.0 / time.duration_since(*last_time).as_secs_f64();
                *last_time = time;
                acc += len;
                println!("{:.0}\t{}\t{}", speed, acc, len);
                future::ready(Some(bytes))
            });
        let socket = tokio_stream::StreamExt::timeout(socket, timeout);
        match socket.try_fold((), |_, _| async move {Ok(())}).await {
            Ok(o) => return Ok(o),
            Err(_) => continue,
        }
    }
}