use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::SystemTime,
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;

type AppState = Arc<RwLock<CurrentState>>;

#[tokio::main]
async fn main() {
    let state = Arc::new(RwLock::new(CurrentState::default()));
    let router = Router::new()
        .route("/", post(handle_sync_event))
        .route("/api/status", get(get_status))
        .route("/ws", get(upgrade_ws))
        .with_state(state)
        .fallback_service(ServeDir::new("./content"));
    axum::Server::bind(&"0.0.0.0:9000".parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}

async fn get_status(State(state): State<AppState>) -> Result<impl IntoResponse, ()> {
    let state = state.read().unwrap();
    #[derive(Serialize)]
    struct Response {
        id: i32,
        filename: Option<String>,
        start_time: f64,
    }
    let avg_start = state
        .start_times
        .iter()
        .copied()
        .reduce(|a, b| a + b)
        .map(|s| s / state.start_times.len() as f64)
        .unwrap_or(current_time());
    Ok(Json(Response {
        id: state.id,
        filename: state.filename.clone(),
        start_time: avg_start,
    }))
}

async fn handle_sync_event(State(app_state): State<AppState>, Json(event): Json<Event>) {
    println!("Received event: {}", event);
    let mut state = app_state.write().unwrap();
    match event {
        Event::MediaStart { id, filename } => {
            if id >= state.id || state.id - id > 100 {
                state.id = id;
                state.filename = Some(filename);
                state.start_times.clear();
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
                // Calculate start time of sequence
                let start_time = current_time() - state.time;
                // Add to queue
                state.start_times.push_back(start_time);
                if state.start_times.len() > 20 {
                    state.start_times.pop_front();
                }
            } else {
                println!("Sync event with wrong id: {} (should be {})", id, state.id);
                state.filename = None;
                state.start_times.clear();
            }
        }
    }
}

async fn upgrade_ws(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|ws| handle_ws(ws))
}
async fn handle_ws(mut ws: WebSocket) {
    while let Some(Ok(msg)) = ws.recv().await {
        if matches!(msg, Message::Close(_)) {
            break;
        }
        // Send current time
        ws.send(Message::Text(current_time().to_string()))
            .await
            .unwrap();
    }
}

/// Get current time in seconds since UNIX epoch
fn current_time() -> f64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
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
impl std::fmt::Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Sync {
                id,
                time,
                latencies,
            } => write!(
                f,
                "Sync(id={}, time={}, latencies={:?})",
                id, time, latencies
            ),
            Event::MediaStart { id, filename } => {
                write!(f, "MediaStart(id={}, filename={})", id, filename)
            }
            Event::MediaStop { id } => write!(f, "MediaStop(id={})", id),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
struct CurrentState {
    filename: Option<String>,
    #[serde(skip)]
    id: i32,
    time: f64,
    #[serde(skip)]
    start_times: VecDeque<f64>,
}
