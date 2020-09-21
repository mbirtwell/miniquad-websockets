use crate::error::{Error, Result};
use crate::event::Event;
use crate::request::Request;
use futures_util::sink::SinkExt;
use miniquad::CustomEventPostBox;
use std::thread;
use tokio::runtime::{Builder, Handle};
use tokio::select;
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::oneshot;
use tokio_tungstenite::connect_async;
pub use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::{Error as TungError, Result as TungResult};

pub struct WebSocketContext<EventType> {
    post_box: CustomEventPostBox<EventType>,
    runtime: Handle,
    #[allow(dead_code)]
    thread_handle: thread::JoinHandle<()>,
    #[allow(dead_code)]
    end_channel: oneshot::Sender<()>,
}
pub struct WebSocketSink(Sender<Message>);

pub fn init<WebSocketId, EventType>(
    post_box: CustomEventPostBox<EventType>,
) -> Result<WebSocketContext<EventType>>
where
    EventType: Send + From<Event<WebSocketId>>,
    WebSocketId: Clone,
{
    let mut runtime = Builder::new().basic_scheduler().enable_io().build()?;
    let handle = runtime.handle().clone();
    let (tx, rx) = oneshot::channel();
    let thread_handle = thread::spawn(move || {
        runtime.block_on(async {
            rx.await.unwrap();
        })
    });
    Ok(WebSocketContext {
        post_box,
        runtime: handle,
        thread_handle,
        end_channel: tx,
    })
}

fn process_recv<WebSocketId, EventType>(
    id: WebSocketId,
    post_box: &CustomEventPostBox<EventType>,
    msg: Option<TungResult<Message>>,
) -> bool
where
    EventType: Send + From<Event<WebSocketId>>,
    WebSocketId: Clone,
{
    match msg {
        Some(Ok(msg)) => {
            post_box.post(Event::message(id, msg));
            true
        }
        Some(Err(TError::ConnectionClosed)) | None => {
            post_box.post(Event::connection_closed(id));
            false
        }
        Some(Err(err)) => {
            post_box.post(Event::error(id, err.into()));
            // TODO: some of these might by non-fatal
            false
        }
    }
}

async fn run_websocket<WebSocketId, EventType>(
    id: WebSocketId,
    request: Request,
    post_box: CustomEventPostBox<EventType>,
) where
    EventType: Send + From<Event<WebSocketId>>,
    WebSocketId: Clone,
{
    let (mut socket, mut to_send_rx) = match connect_async(request).await {
        Ok((socket, response)) => {
            let (tx, rx) = channel(2);
            post_box.post(Event::connected(id.clone(), WebSocketSink(tx), response));
            (socket, rx)
        }
        Err(err) => {
            post_box.post(Event::connection_failed(id, err.into()));
            return;
        }
    };

    loop {
        select! {
            rx_msg = socket.next() => {
                if !process_recv(id.clone(), &post_box, rx_msg) {
                    break;
                }
            }
            tx_msg = to_send_rx.next() => {
                match tx_msg {
                    Some(msg) => match socket.send(msg).await {
                        Ok(()) => {},
                        Err(err) => {
                            post_box.post(Event::error(id, err.into()));
                            // TODO: some of these might by non-fatal
                            break;
                        }
                    },
                    None => break,
                }
            }
        }
    }
}

impl<EventType> WebSocketContext<EventType> {
    pub fn start_connect<WebSocketId>(&mut self, id: WebSocketId, request: Request)
    where
        EventType: Send + From<Event<WebSocketId>> + 'static,
        WebSocketId: Send + Clone + 'static,
    {
        let post_box = self.post_box.clone();
        self.runtime.spawn(async {
            run_websocket(id, request, post_box).await;
        });
    }
}

impl From<TError> for Error {
    fn from(err: TError) -> Self {
        match err {
            TError::ConnectionClosed => Error::ConnectionClosed,
            TError::AlreadyClosed => Error::AlreadyClosed,
            TError::Io(err) => Error::Io(err),
            #[cfg(feature = "tls")]
            TError::Tls(err) => Error::Tls(err),
            TError::Capacity(msg) => Error::Capacity(msg),
            TError::Protocol(msg) => Error::Protocol(msg),
            TError::SendQueueFull(msg) => Error::SendQueueFull(msg),
            TError::Utf8 => Error::Utf8,
            TError::Url(msg) => Error::Url(msg),
            TError::Http(code) => Error::Http(code),
            TError::HttpFormat(err) => Error::HttpFormat(err),
        }
    }
}
