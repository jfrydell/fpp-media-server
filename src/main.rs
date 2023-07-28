use std::sync::{Arc, RwLock};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

type AppState = (Arc<RwLock<CurrentState>>, broadcast::Sender<()>);

#[tokio::main]
async fn main() {
    let (update_sender, _) = broadcast::channel(1);
    let state = Arc::new(RwLock::new(CurrentState::default()));
    let router = Router::new()
        .route("/", post(handle_sync_event))
        .route("/ws", get(upgrade_ws))
        .with_state((state, update_sender))
        .fallback_service(ServeDir::new("./content"));
    axum::Server::bind(&"0.0.0.0:9000".parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}

async fn handle_sync_event(State(app_state): State<AppState>, Json(event): Json<Event>) {
    let mut state = app_state.0.write().unwrap();
    match event {
        Event::MediaStart { id, filename } => {
            if id >= state.id || state.id - id > 100 {
                state.id = id;
                state.filename = Some(filename);
            }
        }
        Event::MediaStop { id } => {
            if id >= state.id || state.id - id > 100 {
                state.id = id;
                state.filename = None;
            }
        }
        Event::Sync {
            id,
            time,
            latencies,
        } => {
            if id == state.id {
                let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
                state.time = time + avg_latency;
            } else {
                println!("Sync event with wrong id: {} (should be {})", id, state.id);
            }
        }
    }
    drop(state);
    println!("Sending update: {:?}", *app_state.0.read().unwrap());
    let _ = app_state.1.send(()); // Ignore error if no one is listening
}

async fn upgrade_ws(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|ws| handle_ws(ws, state))
}
async fn handle_ws(mut ws: WebSocket, state: AppState) {
    let mut rx = state.1.subscribe();
    while let Ok(()) = rx.recv().await {
        let msg = {
            let state = state.0.read().unwrap();
            Message::Text(serde_json::to_string(&*state).unwrap())
        };
        ws.send(msg).await.unwrap();
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Event {
    Sync {
        id: i32,
        time: f64,
        latencies: [f64; 3],
    },
    MediaStart {
        id: i32,
        filename: String,
    },
    MediaStop {
        id: i32,
    },
}

#[derive(Debug, Clone, Default, Serialize)]
struct CurrentState {
    filename: Option<String>,
    #[serde(skip)]
    id: i32,
    time: f64,
}
