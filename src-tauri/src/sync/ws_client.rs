use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::types::WsMessage;

pub struct WsClient {
    outgoing_tx: mpsc::Sender<String>,
    incoming_tx: broadcast::Sender<WsMessage>,
    shutdown_tx: mpsc::Sender<()>,
}

impl WsClient {
    pub async fn connect(ws_url: &str) -> Result<Self, String> {
        let url = url::Url::parse(ws_url).map_err(|e| format!("Invalid WS URL: {}", e))?;

        let (ws_stream, _) = connect_async(url.as_str())
            .await
            .map_err(|e| format!("WebSocket connect failed: {}", e))?;

        let (mut ws_sink, mut ws_stream_rx) = ws_stream.split();

        let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<String>(64);
        let (incoming_tx, _) = broadcast::channel::<WsMessage>(64);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let incoming_tx_clone = incoming_tx.clone();

        // Send task: forwards outgoing messages to the WebSocket
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = outgoing_rx.recv() => {
                        if ws_sink.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        let _ = ws_sink.close().await;
                        break;
                    }
                }
            }
        });

        // Receive task: reads from WebSocket and broadcasts parsed messages
        tokio::spawn(async move {
            while let Some(result) = ws_stream_rx.next().await {
                match result {
                    Ok(Message::Text(text)) => {
                        if let Ok(msg) = serde_json::from_str::<WsMessage>(&text) {
                            let _ = incoming_tx_clone.send(msg);
                        }
                    }
                    Ok(Message::Close(_)) => break,
                    Err(_) => break,
                    _ => {}
                }
            }
            println!("[ClipSlot] WebSocket receive loop ended");
        });

        println!("[ClipSlot] WebSocket connected");

        Ok(Self {
            outgoing_tx,
            incoming_tx,
            shutdown_tx,
        })
    }

    pub async fn send(&self, msg: &WsMessage) -> Result<(), String> {
        let json = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        self.outgoing_tx
            .send(json)
            .await
            .map_err(|e| format!("Send failed: {}", e))
    }

    pub fn subscribe(&self) -> broadcast::Receiver<WsMessage> {
        self.incoming_tx.subscribe()
    }

    pub async fn disconnect(&self) {
        let _ = self.shutdown_tx.send(()).await;
    }
}
