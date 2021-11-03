//! This example leverages `BytesCodec` to create a UDP client and server which
//! speak a custom protocol.
//!
//! Here we're using the codec from `tokio-codec` to convert a UDP socket to a stream of
//! client messages. These messages are then processed and returned back as a
//! new message with a new destination. Overall, we then use this to construct a
//! "ping pong" pair where two sockets are sending messages back and forth.

#![warn(rust_2018_idioms)]

use std::convert::TryInto;
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

    let throttle = env::args()
        .nth(4)
        .unwrap_or_else(|| "0.05".to_string());

    let timeout = env::args()
        .nth(5)
        .unwrap_or_else(|| "0.01".to_string());

    let throttle = Duration::from_secs_f64(throttle.parse().unwrap());
    let timeout = Duration::from_secs_f64(timeout.parse().unwrap());
    
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
    let socket = UdpSocket::bind(from_addr.clone()).await?;
    socket.set_broadcast(true)?;
    let mut socket = UdpFramed::new(socket, BytesCodec::new());
    loop {
        let reader_stream = ReaderStream::with_capacity(
            File::open("images/image.png").await?,
            1024 * 50
        )
            // .inspect(|r| {dbg!(r.as_deref().map(|b| b.len()));})
            .chunks(4)
            .enumerate()
            .scan((0, Instant::now()), |(acc, last_time), (i, bytes)| {
                let mut bytes: BytesMut = bytes.into_iter().map(Result::unwrap).flatten().collect();

                let mut b = [0; 16];
                let n = i.to_le_bytes();
                b[..n.len()].copy_from_slice(&n);
                bytes.extend_from_slice(&b);

                let time = Instant::now();
                let len = bytes.len();
                // let speed = len as f64 / 1024.0 / 1024.0 / time.duration_since(*last_time).as_secs_f64();
                *last_time = time;
                *acc += len;
                println!("{}", i);
                future::ready(Some(bytes.freeze()))
            })
            .map(|b| Ok((b, to_addr)));
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
    let socket = UdpSocket::bind(to_addr.clone()).await?;
    socket.set_broadcast(true)?;
    let mut acc = 0;
    let mut last_i = 0;
    loop {
        let stream = UdpFramed::new(
            &socket,
            BytesCodec::new()
        )
            .map(|e| e.unwrap().0)
            .scan((0, Instant::now()), |(_, last_time), mut bytes| {
                let len = bytes.len();

                let b = bytes.split_off(len - 16);
                let i = usize::from_le_bytes(b[..std::mem::size_of::<usize>()].try_into().unwrap());
                let diff = i - last_i;
                last_i = i;

                let time = Instant::now();
                // let speed = len as f64 / 1024.0 / 1024.0 / time.duration_since(*last_time).as_secs_f64();
                *last_time = time;
                acc += len;
                println!("{}\t{}", i, if diff != 1 {diff.to_string()} else {"".to_string()});
                future::ready(Some(bytes))
            });
        let stream = tokio_stream::StreamExt::timeout(stream, timeout);
        match stream.try_fold((), |_, _| async move {Ok(())}).await {
            Ok(o) => return Ok(o),
            Err(e) => println!("{}", e),
        }
    }
}