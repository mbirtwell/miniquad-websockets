use crate::error::Error;
use crate::request::Response;
use crate::{Message, WebSocketSink};
use std::fmt::{self, Debug, Formatter};

pub struct WebSocketEvent<WebSocketId> {
    pub id: WebSocketId,
    pub kind: WebSocketEventKind,
}

pub enum WebSocketEventKind {
    Connected(WebSocketSink, Response),
    ConnectionFailed(Error),
    Message(Message),
    ConnectionClosed,
    Error(Error),
}

impl Debug for WebSocketEventKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            WebSocketEventKind::Connected(_, r) => {
                write!(f, "WebSocketEventKind::Connected(..., {:?})", r)
            }
            WebSocketEventKind::ConnectionFailed(err) => {
                write!(f, "WebSocketEventKind::ConnectionFailed({:?})", err)
            }
            WebSocketEventKind::Message(msg) => write!(f, "WebSocketEventKind::Message({:?})", msg),
            WebSocketEventKind::ConnectionClosed => {
                write!(f, "WebSocketEventKind::ConnectionClosed")
            }
            WebSocketEventKind::Error(err) => write!(f, "WebSocketEventKind::Error({:?})", err),
        }
    }
}

impl<WebSocketId> WebSocketEvent<WebSocketId> {
    pub fn connected(id: WebSocketId, sink: WebSocketSink, response: Response) -> Self {
        Self {
            id,
            kind: WebSocketEventKind::Connected(sink, response),
        }
    }

    pub fn connection_failed(id: WebSocketId, err: Error) -> Self {
        Self {
            id,
            kind: WebSocketEventKind::ConnectionFailed(err),
        }
    }

    pub fn message(id: WebSocketId, msg: Message) -> Self {
        Self {
            id,
            kind: WebSocketEventKind::Message(msg),
        }
    }

    pub fn connection_closed(id: WebSocketId) -> Self {
        Self {
            id,
            kind: WebSocketEventKind::ConnectionClosed,
        }
    }

    pub fn error(id: WebSocketId, err: Error) -> Self {
        Self {
            id,
            kind: WebSocketEventKind::Error(err),
        }
    }
}
