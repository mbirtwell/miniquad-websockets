use crate::error::Error;
use crate::request::Response;
use crate::{Message, WebSocketSink};

pub struct Event<WebSocketId> {
    pub id: WebSocketId,
    pub kind: EventKind,
}

pub enum EventKind {
    Connected(WebSocketSink, Response),
    ConnectionFailed(Error),
    Message(Message),
    ConnectionClosed,
    Error(Error),
}

impl<WebSocketId> Event<WebSocketId> {
    pub fn connected(id: WebSocketId, sink: WebSocketSink, response: Response) -> Self {
        Self {
            id,
            kind: EventKind::Connected(sink, response),
        }
    }

    pub fn connection_failed(id: WebSocketId, err: Error) -> Self {
        Self {
            id,
            kind: EventKind::ConnectionFailed(err),
        }
    }

    pub fn message(id: WebSocketId, msg: Message) -> Self {
        Self {
            id,
            kind: EventKind::Message(msg),
        }
    }

    pub fn connection_closed(id: WebSocketId) -> Self {
        Self {
            id,
            kind: EventKind::ConnectionClosed,
        }
    }

    pub fn error(id: WebSocketId, err: Error) -> Self {
        Self {
            id,
            kind: EventKind::Error(err),
        }
    }
}
