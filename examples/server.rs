use std::collections::HashMap;
use std::io;

use example_interface::{ClientState, Color, ConnectionId, MousePos};
use futures_util::SinkExt;
use nanoserde::{DeJson, SerJson};
use rand::{thread_rng, Rng};
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio::stream::StreamExt;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::spawn;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{self, accept_async, tungstenite};

mod example_interface;

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

impl Color {
    fn random() -> Self {
        let mut rng = thread_rng();
        Color {
            r: rng.gen_range(0., 1.),
            g: rng.gen_range(0., 1.),
            b: rng.gen_range(0., 1.),
        }
    }
}

#[derive(Debug)]
struct ClientEvent {
    id: ConnectionId,
    kind: ClientEventKind,
}

#[derive(Debug)]
enum ClientEventKind {
    NewConnection(MousePos, Color, oneshot::Sender<Vec<ClientState>>),
    Update(MousePos),
    Disconnection,
}

async fn distribute(mut rx: mpsc::Receiver<ClientEvent>, tx: broadcast::Sender<ClientState>) {
    let mut data: HashMap<ConnectionId, ClientState> = HashMap::new();
    loop {
        match rx.next().await {
            None => break,
            Some(ClientEvent {
                id,
                kind: ClientEventKind::NewConnection(pos, color, initial_tx),
            }) => {
                println!(
                    "Districute handling new connection id: {:?} color: {:?}",
                    id, color
                );
                initial_tx
                    .send(data.values().map(|x| x.clone()).collect())
                    .unwrap();
                let new_state = ClientState { id, color, pos };
                data.insert(id, new_state.clone());
                tx.send(new_state).unwrap();
            }
            Some(ClientEvent {
                id,
                kind: ClientEventKind::Update(pos),
            }) => {
                let v = data.get_mut(&id).unwrap();
                v.pos = pos;
                tx.send(v.clone()).unwrap();
            }
            Some(ClientEvent {
                id,
                kind: ClientEventKind::Disconnection,
            }) => {
                data.remove(&id);
            }
        }
    }
}

fn deserialize_msg(msg: Message) -> Option<MousePos> {
    match msg {
        Message::Text(s) => Some(MousePos::deserialize_json(&s).unwrap()),
        Message::Close(cf) => {
            println!("Close frame: {:?}", cf);
            None
        }
        _ => panic!("Unsupported message type. {:?}", msg),
    }
}

async fn process_rx(
    id: ConnectionId,
    rx_update: Option<tungstenite::Result<Message>>,
    distribute_tx: &mut mpsc::Sender<ClientEvent>,
) -> bool {
    match rx_update {
        Some(Ok(rx_update)) => {
            if let Some(pos) = deserialize_msg(rx_update) {
                distribute_tx
                    .send(ClientEvent {
                        id,
                        kind: ClientEventKind::Update(pos),
                    })
                    .await
                    .unwrap();
            }
            true
        }
        Some(Err(rx_err)) => {
            println!("Connnection {:?} closed with error {}", id, rx_err);
            distribute_tx
                .send(ClientEvent {
                    id,
                    kind: ClientEventKind::Disconnection,
                })
                .await
                .unwrap();
            false
        }
        None => {
            println!("Connnection {:?} closed cleanly", id);
            distribute_tx
                .send(ClientEvent {
                    id,
                    kind: ClientEventKind::Disconnection,
                })
                .await
                .unwrap();
            false
        }
    }
}

async fn process_socket(
    socket: TcpStream,
    mut distribute_tx: mpsc::Sender<ClientEvent>,
    mut distribute_rx: broadcast::Receiver<ClientState>,
) {
    let id = ConnectionId(NEXT_ID.fetch_add(1, Ordering::AcqRel));
    let color = Color::random();
    println!("New connection id: {:?} color: {:?}", id, color);
    let mut ws = accept_async(socket).await.unwrap();
    let initial = ws.next().await.unwrap().unwrap();
    let pos = deserialize_msg(initial).unwrap();
    let (tx, rx) = oneshot::channel();
    distribute_tx
        .send(ClientEvent {
            id,
            kind: ClientEventKind::NewConnection(pos, color, tx),
        })
        .await
        .unwrap();
    for datum in rx.await.unwrap() {
        ws.send(Message::Text(datum.serialize_json()))
            .await
            .unwrap();
    }
    loop {
        select! {
            rx_update = ws.next() => {
                if !process_rx(id, rx_update, &mut distribute_tx).await {
                    break;
                }
            }
            tx_update = distribute_rx.next() => {
                match tx_update {
                    Some(x) => ws.send(Message::Text(x.unwrap().serialize_json())).await.unwrap(),
                    None => break,
                };
            }
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut listener = TcpListener::bind("0.0.0.0:8080").await?;
    let (ingress_tx, ingress_rx) = mpsc::channel(10);
    let (egress_tx, egress_rx) = broadcast::channel(10);
    drop(egress_rx);
    let egress_tx2 = egress_tx.clone();

    spawn(distribute(ingress_rx, egress_tx));

    println!("Running");
    loop {
        let (socket, _) = listener.accept().await?;
        spawn(process_socket(
            socket,
            ingress_tx.clone(),
            egress_tx2.subscribe(),
        ));
    }
}
