use std::fs::File;
use std::io::BufReader;
use std::sync::{
    atomic::{AtomicU16, Ordering as ordering},
    Arc,
};
use std::thread;

use axum::{extract::State, routing::post, Json, Router};
use gsi_cs2::Body;

async fn update(State(kills): State<Arc<AtomicU16>>, data: Json<Body>) {
    let map = data.map.as_ref();
    if let None = map {
        println!("You need to load map");
        return;
    }

    let player_data = data.player.as_ref();
    if let None = player_data {
        return;
    }

    let player = player_data.unwrap();
    let state = player.state.as_ref().unwrap();

    let current_kills = state.round_kills;
    // let current_headshots = state.round_killhs;
    let original_kills = kills.load(ordering::Relaxed);

    if current_kills != original_kills {
        kills.store(current_kills, ordering::Relaxed);
    }
    print!("\x1B[2J\x1B[1;1H"); //clear

    if current_kills == original_kills + 1 {
        let sound_num = if current_kills > 5 { 5 } else { current_kills };

        thread::spawn(move || {
            let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
            let file = File::open(format!("sounds/varolant/{}.mp3", sound_num)).unwrap();
            let source = rodio::Decoder::new(BufReader::new(file)).unwrap();

            let sink = rodio::Sink::try_new(&stream_handle).unwrap();
            sink.append(source);
            sink.set_volume(0.3);
            sink.play();
            sink.sleep_until_end();
        });
    }

    println!("Kills: {}", current_kills);
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let kills = Arc::new(AtomicU16::new(0));

    let app = Router::new()
        .route("/", post(update))
        .with_state(kills.clone());

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
