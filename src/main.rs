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
// use futures::future::FusedFuture;
use std::env;
use std::error::Error;
use std::time::{Duration, Instant};

// mod timeout_stream;
// mod timeout;
// use timeout_stream::TimeoutStream;

// #[tokio::main]
// async fn old_main() -> Result<(), Box<dyn Error>> {
//     let a_addr = env::args()
//         .nth(1)
//         .unwrap_or_else(|| "127.0.0.1:8000".to_string());
    
//     let b_addr = env::args()
//         .nth(2)
//         .unwrap_or_else(|| "127.0.0.1:8001".to_string());

//     // Bind both our sockets and then figure out what ports we got.
//     let a = UdpSocket::bind(&a_addr).await?;
//     // let a_2 = UdpSocket::bind(&addr).await?;
//     let b = UdpSocket::bind(&b_addr).await?;

//     let b_addr = b.local_addr()?;

//     let mut a = UdpFramed::new(a, BytesCodec::new());
//     // let mut a_2 = UdpFramed::new(a_2, BytesCodec::new());
//     let b = UdpFramed::new(b, BytesCodec::new());

//     let file = File::open("images/image.png").await?;
//     // let file_2 = file.try_clone().await?;
//     // let file_2 = File::open("images/image.png").await?;
//     let total = file.metadata().await?.len();
//     let mut reader_stream = ReaderStream::with_capacity(file, 1024 * 9)
//         .map(|b| b.map(|b| (b, b_addr)))
//         // .scan(0, |acc, x| {
//         //     *acc += x.as_ref().map(|(b, _)| b.len()).unwrap_or(0);
//         //     println!("first: {}", *acc as f64 / 1024.0 / 1024.0);
//         //     future::ready(Some(x))
//         // })
//         ;

//     // let mut reader_stream_2 = ReaderStream::with_capacity(file_2, 1024 * 9)
//         // .map(|b| b.map(|b| (b, b_addr)))
//         // .scan(0, |acc, x| {
//         //     *acc += x.as_ref().map(|(b, _)| b.len()).unwrap_or(0);
//         //     println!("second: {}", *acc as f64 / 1024.0 / 1024.0);
//         //     future::ready(Some(x))
//         // })
//         // ;

//     // let mut a = a.fanout(a_2);
//     let mut a = a.send_all(&mut reader_stream);
//     // let mut a_2 = a_2.send_all(&mut reader_stream_2);

//     let b = b
//         // .scan(0, |acc, x| {
//         //     *acc += x.as_ref().map(|(b, _)| b.len()).unwrap_or(0);
//         //     println!("total: {}", *acc as f64 / 1024.0 / 1024.0);
//         //     future::ready(Some(x))
//         // })
//         .map(|e| e.unwrap().0);

//         // .fold((0, Instant::now()), |(acc, start), e| async move {
//         //     let total = acc + e.len();
//         //     let mb_per_sec = total as f64 / start.elapsed().as_secs_f64() / 1024.0 / 1024.0;
//         //     dbg!(mb_per_sec);
//         //     (total, start)
//         // });

//     let b = tokio_stream::StreamExt::timeout(b, Duration::from_secs_f64(1.0));
//     // let b = b
//     //     .try_fold(0, |acc, start| async move {
//     //         let total = acc + start.len();
//     //         dbg!(total / 1024 / 1024);
//     //         Ok(total)
//     //     });
//     let b = b
//         .try_fold((), |_, _| async move {Ok(())});

//     let start = Instant::now();
//     // let (a, a_2, b) = tokio::join!(a, a_2, b);
//     let (mut is_a, mut is_a_2) = (false, false);
//     tokio::pin!(b);
//     loop {
//         tokio::select! {
//             a = &mut a, if !is_a => {
//                 dbg!(a.unwrap());
//                 is_a = true;
//             },
//             // a_2 = &mut a_2, if !is_a_2 => {
//             //     dbg!(a_2.unwrap());
//             //     is_a_2 = true;
//             // },
//             b = &mut b, if !b.is_terminated() => {
//                 dbg!(b).expect_err("Should elapse");
//             },
//             else => {
//                 break;
//             }
//         }
//     }

//     let mb_per_sec = total as f64 / (start.elapsed().as_secs_f64() - 1.0) / 1024.0 / 1024.0;
//     dbg!(mb_per_sec);

//     // a.unwrap();
//     // a_2.unwrap();
//     // b.expect_err("Should be Elapsed");
//     Ok(())
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let command = env::args()
        .nth(1)
        .unwrap_or_else(|| "both".to_string());

    let a_addr = env::args()
        .nth(2)
        .unwrap_or_else(|| "127.0.0.1:8000".to_string());
    
    let b_addr = env::args()
        .nth(3)
        .unwrap_or_else(|| "127.0.0.1:8001".to_string());
    
    
    match command.as_str() {
        "both" => both(a_addr, b_addr).await,
        "sink" => sink(a_addr, b_addr).await,
        "stream" => stream(b_addr).await,
        _ => panic!("wrong command")
    }
}

async fn both(a_addr: String, b_addr: String) -> Result<(), Box<dyn Error>> {
    let file = File::open("images/image.png").await?;
    let total = file.metadata().await?.len();
    let start = Instant::now();

    let a = udp_sink(a_addr, b_addr.parse()?);
    let b = udp_stream(b_addr);
    let (a, b) = tokio::join!(a, b);
    
    let elapsed = dbg!(start.elapsed().as_secs_f64());
    let mb_per_sec = total as f64 / (elapsed - 1.0) / 1024.0 / 1024.0;
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

async fn sink(a_addr: String, b_addr: String) -> Result<(), Box<dyn Error>> {
    let file = File::open("images/image.png").await?;
    let total = file.metadata().await?.len();
    let start = Instant::now();

    let a = udp_sink(a_addr, b_addr.parse()?).await;
    
    let elapsed = dbg!(start.elapsed().as_secs_f64());
    let mb_per_sec = total as f64 / elapsed / 1024.0 / 1024.0;
    dbg!(mb_per_sec);
    a
}

async fn stream(b_addr: String) -> Result<(), Box<dyn Error>> {
    let file = File::open("images/image.png").await?;
    let total = file.metadata().await?.len();
    let start = Instant::now();

    let b = udp_stream(b_addr).await;
    
    let elapsed = dbg!(start.elapsed().as_secs_f64());
    let mb_per_sec = total as f64 / (elapsed - 5.0) / 1024.0 / 1024.0;
    dbg!(mb_per_sec);
    match b {
        Ok(_) => unreachable!("Should time out"),
        Err(_) => Ok(())
    }
}

async fn udp_sink<A: ToSocketAddrs>(
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