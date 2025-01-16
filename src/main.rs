mod whisper;

use std::net::SocketAddr;
use std::sync::Mutex;

use dasp_sample::Sample;
use futures_util::{SinkExt, StreamExt};
use once_cell::sync::Lazy;
use ringbuffer::{AllocRingBuffer, RingBuffer};
use samplerate::{convert, ConverterType};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::Duration;
use tokio::time::{sleep, Instant};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error, Result},
};

static SPEECH_BUF: Lazy<Mutex<AllocRingBuffer<f32>>> =
    Lazy::new(|| Mutex::new(AllocRingBuffer::new(16000 * 30))); // 30s

fn transcribe_in_background() {
    std::thread::spawn(|| {
        let mut speech_buf = SPEECH_BUF.lock().unwrap();
        let samples = speech_buf.to_vec();

        let min_samples = (1.0 * 16_000.0) as usize;
        if samples.len() < min_samples {
            println!("Less than 1s. Skipping...");
            return;
        }

        if let Some(text) = whisper::transcribe(&samples) {
            println!("text: {}", text);
        }
        speech_buf.clear();
    });
}

async fn accept_connection(peer: SocketAddr, stream: TcpStream) {
    if let Err(e) = handle_connection(peer, stream).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => tracing::error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream) -> Result<()> {
    let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

    tracing::info!("New WebSocket connection: {}", peer);

    loop {
        let time = Instant::now();
        tokio::select! {
            _ = sleep(Duration::from_millis(1500)) => {
                tracing::info!("timed out, starting transcription in background");
                transcribe_in_background();
            },
            msg = ws_stream.next() => {
                if let Some(msg) = msg {
                    let silence_dur = Instant::now() - time;
                    let msg = msg?;
                    if msg.is_binary() {
                        tracing::info!("{} bytes incoming", msg.len());
                        let data = msg.into_data();

                        //inserted silence???!?!?! no way
                        let silence_buffer: Vec<f32> = vec![0.0; (16 * silence_dur.as_millis()).try_into().unwrap()];
                        SPEECH_BUF.lock().unwrap().extend(silence_buffer);

                        let data: Vec<i16> = data
                           .chunks_exact(2)
                           .into_iter()
                           .map(|a| i16::from_ne_bytes([a[0], a[1]]))
                           .collect();

                        let samples: Vec<f32> = data
                            .iter()
                            .map(|s| s.to_float_sample().to_sample())
                            .collect();
                        let samples = convert(48000, 16000, 1, ConverterType::SincBestQuality, &samples)
                            .expect("sample conversion failed???");

                        SPEECH_BUF.lock().unwrap().extend(samples.clone());

                        ws_stream.send(Message::text("ok")).await?;
                    } else {
                        tracing::warn!("Received text message instead of binary...?");
                    }
                } else {
                  break Ok(());
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};
    // Configure a `tracing` subscriber that logs traces emitted by the chat
    // server.
    tracing_subscriber::fmt()
        // Filter what traces are displayed based on the RUST_LOG environment
        // variable.
        //
        // Traces emitted by the example code will always be displayed. You
        // can set `RUST_LOG=tokio=trace` to enable additional traces emitted by
        // Tokio itself.
        .with_env_filter(EnvFilter::from_default_env().add_directive("debug".parse()?))
        // Log events when `tracing` spans are created, entered, exited, or
        // closed. When Tokio's internal tracing support is enabled (as
        // described above), this can be used to track the lifecycle of spawned
        // tasks on the Tokio runtime.
        .with_span_events(FmtSpan::FULL)
        // Set this subscriber as the default, to collect all traces emitted by
        // the program.
        .init();

    let model_path = std::env::args()
        .nth(1)
        .expect("Please specify path to model");

    tracing::info!("Starting");

    let listener = TcpListener::bind("127.0.0.1:4649").await?;

    whisper::init(&model_path);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        tracing::info!("Peer address: {}", peer);

        tokio::spawn(accept_connection(peer, stream));
    }

    Ok(())
}
