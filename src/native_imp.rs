use crate::error::{Error, Result};
use crate::event::WebSocketEvent;
use crate::request::Request;
use crate::IntoClientRequest;
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
pub struct WebSocketSink(Handle, Sender<Message>);

pub fn init<WebSocketId, EventType>(
    post_box: CustomEventPostBox<EventType>,
) -> Result<WebSocketContext<EventType>>
where
    EventType: Send + From<WebSocketEvent<WebSocketId>>,
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
    EventType: Send + From<WebSocketEvent<WebSocketId>>,
    WebSocketId: Clone,
{
    match msg {
        Some(Ok(msg)) => {
            post_box.post(WebSocketEvent::message(id, msg));
            true
        }
        Some(Err(TungError::ConnectionClosed)) | None => {
            post_box.post(WebSocketEvent::connection_closed(id));
            false
        }
        Some(Err(err)) => {
            post_box.post(WebSocketEvent::error(id, err.into()));
            // TODO: some of these might by non-fatal
            false
        }
    }
}

async fn run_websocket<WebSocketId, EventType>(
    id: WebSocketId,
    request: Request,
    handle: Handle,
    post_box: CustomEventPostBox<EventType>,
) where
    EventType: Send + From<WebSocketEvent<WebSocketId>>,
    WebSocketId: Clone,
{
    let (mut socket, mut to_send_rx) = match connect_async(request).await {
        Ok((socket, response)) => {
            let (tx, rx) = channel(2);
            post_box.post(WebSocketEvent::connected(
                id.clone(),
                WebSocketSink(handle, tx),
                response,
            ));
            (socket, rx)
        }
        Err(err) => {
            post_box.post(WebSocketEvent::connection_failed(id, err.into()));
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
                            post_box.post(WebSocketEvent::error(id, err.into()));
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
    pub fn start_connect<WebSocketId, R>(&mut self, id: WebSocketId, request: R) -> Result<()>
    where
        EventType: Send + From<WebSocketEvent<WebSocketId>> + 'static,
        WebSocketId: Send + Clone + 'static,
        R: IntoClientRequest,
    {
        let post_box = self.post_box.clone();
        let request = request.into_client_request()?;
        let handle = self.runtime.clone();
        self.runtime.spawn(async move {
            run_websocket(id, request, handle, post_box).await;
        });
        Ok(())
    }
}

impl WebSocketSink {
    pub fn send(&mut self, msg: Message) -> Result<()> {
        let sender = &mut self.1;
        self.0
            .block_on(async { sender.send(msg).await })
            .map_err(|_| Error::AlreadyClosed)
    }
}

impl From<TungError> for Error {
    fn from(err: TungError) -> Self {
        match err {
            TungError::ConnectionClosed => Error::ConnectionClosed,
            TungError::AlreadyClosed => Error::AlreadyClosed,
            TungError::Io(err) => Error::Io(err),
            #[cfg(feature = "tls")]
            TungError::Tls(err) => Error::Tls(err),
            TungError::Capacity(msg) => Error::Capacity(msg),
            TungError::Protocol(msg) => Error::Protocol(msg),
            TungError::SendQueueFull(msg) => Error::SendQueueFull(msg),
            TungError::Utf8 => Error::Utf8,
            TungError::Url(msg) => Error::Url(msg),
            TungError::Http(code) => Error::Http(code),
            TungError::HttpFormat(err) => Error::HttpFormat(err),
        }
    }
}
