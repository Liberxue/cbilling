use std::time::Duration;

use crossterm::event::{Event, EventStream, KeyEventKind, MouseEventKind};
use futures::StreamExt;
use tokio::sync::mpsc;

use cbilling::service::BillingData;

pub enum AppEvent {
    Key(crossterm::event::KeyEvent),
    ScrollUp(u16),
    ScrollDown(u16),
    Tick,
    DataResult {
        provider: String,
        result: Result<BillingData, String>,
        is_prev: bool,
    },
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    pub tx: mpsc::UnboundedSender<AppEvent>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let tx_events = tx.clone();
        tokio::spawn(async move {
            let mut stream = EventStream::new();
            while let Some(Ok(event)) = stream.next().await {
                let app_event = match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => Some(AppEvent::Key(key)),
                    Event::Mouse(mouse) => match mouse.kind {
                        MouseEventKind::ScrollUp => Some(AppEvent::ScrollUp(3)),
                        MouseEventKind::ScrollDown => Some(AppEvent::ScrollDown(3)),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(evt) = app_event {
                    if tx_events.send(evt).is_err() {
                        break;
                    }
                }
            }
        });

        let tx_tick = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tick_rate);
            loop {
                interval.tick().await;
                if tx_tick.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        });

        Self { rx, tx }
    }

    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }
}
