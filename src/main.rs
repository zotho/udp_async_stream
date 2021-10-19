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
use tokio::net::unix::SocketAddr;
use tokio_stream;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;
use tokio_util::io::ReaderStream;

use bytes::Bytes;
use futures::{SinkExt, StreamExt, TryStreamExt, future::FusedFuture};
use futures::Stream;
use std::env;
use std::error::Error;
use std::time::{Duration, Instant};

// mod timeout_stream;
// mod timeout;
// use timeout_stream::TimeoutStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let a_addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8000".to_string());
    
    let b_addr = env::args()
        .nth(2)
        .unwrap_or_else(|| "127.0.0.1:8001".to_string());

    // Bind both our sockets and then figure out what ports we got.
    let a = UdpSocket::bind(&a_addr).await?;
    // let a_2 = UdpSocket::bind(&addr).await?;
    let b = UdpSocket::bind(&b_addr).await?;

    let b_addr = b.local_addr()?;

    let mut a = UdpFramed::new(a, BytesCodec::new());
    // let mut a_2 = UdpFramed::new(a_2, BytesCodec::new());
    let b = UdpFramed::new(b, BytesCodec::new());

    let file = File::open("images/image.png").await?;
    // let file_2 = file.try_clone().await?;
    // let file_2 = File::open("images/image.png").await?;
    let total = file.metadata().await?.len();
    let mut reader_stream = ReaderStream::with_capacity(file, 1024 * 9)
        .map(|b| b.map(|b| (b, b_addr)))
        // .scan(0, |acc, x| {
        //     *acc += x.as_ref().map(|(b, _)| b.len()).unwrap_or(0);
        //     println!("first: {}", *acc as f64 / 1024.0 / 1024.0);
        //     future::ready(Some(x))
        // })
        ;

    // let mut reader_stream_2 = ReaderStream::with_capacity(file_2, 1024 * 9)
        // .map(|b| b.map(|b| (b, b_addr)))
        // .scan(0, |acc, x| {
        //     *acc += x.as_ref().map(|(b, _)| b.len()).unwrap_or(0);
        //     println!("second: {}", *acc as f64 / 1024.0 / 1024.0);
        //     future::ready(Some(x))
        // })
        // ;

    // let mut a = a.fanout(a_2);
    let mut a = a.send_all(&mut reader_stream);
    // let mut a_2 = a_2.send_all(&mut reader_stream_2);

    let b = b
        // .scan(0, |acc, x| {
        //     *acc += x.as_ref().map(|(b, _)| b.len()).unwrap_or(0);
        //     println!("total: {}", *acc as f64 / 1024.0 / 1024.0);
        //     future::ready(Some(x))
        // })
        .map(|e| e.unwrap().0);

        // .fold((0, Instant::now()), |(acc, start), e| async move {
        //     let total = acc + e.len();
        //     let mb_per_sec = total as f64 / start.elapsed().as_secs_f64() / 1024.0 / 1024.0;
        //     dbg!(mb_per_sec);
        //     (total, start)
        // });

    let b = tokio_stream::StreamExt::timeout(b, Duration::from_secs_f64(1.0));
    // let b = b
    //     .try_fold(0, |acc, start| async move {
    //         let total = acc + start.len();
    //         dbg!(total / 1024 / 1024);
    //         Ok(total)
    //     });
    let b = b
        .try_fold((), |_, _| async move {Ok(())});

    let start = Instant::now();
    // let (a, a_2, b) = tokio::join!(a, a_2, b);
    let (mut is_a, mut is_a_2) = (false, false);
    tokio::pin!(b);
    loop {
        tokio::select! {
            a = &mut a, if !is_a => {
                dbg!(a.unwrap());
                is_a = true;
            },
            // a_2 = &mut a_2, if !is_a_2 => {
            //     dbg!(a_2.unwrap());
            //     is_a_2 = true;
            // },
            b = &mut b, if !b.is_terminated() => {
                dbg!(b).expect_err("Should elapse");
            },
            else => {
                break;
            }
        }
    }

    let mb_per_sec = total as f64 / (start.elapsed().as_secs_f64() - 1.0) / 1024.0 / 1024.0;
    dbg!(mb_per_sec);

    // a.unwrap();
    // a_2.unwrap();
    // b.expect_err("Should be Elapsed");
    Ok(())
}

async fn udp_stream_sink<A: ToSocketAddrs>(
    from_addr: A,
    to_addr: A,
) -> Result<(impl futures::Sink<(Bytes, std::net::SocketAddr)>, impl std::future::Future, impl futures::TryStream), Box<dyn Error>> {
    let from = UdpSocket::bind(from_addr).await?;
    let to = UdpSocket::bind(to_addr).await?;
    let to_addr = to.local_addr()?;
    let mut a = UdpFramed::new(from, BytesCodec::new());
    let file = File::open("images/image.png").await?;
    let mut reader_stream = ReaderStream::with_capacity(file, 1024 * 9)
        .map(move |b| b.map(|b| (b, to_addr)));
    let a_2 = a.send_all(&mut reader_stream);
    Ok((a, a_2, reader_stream))
}