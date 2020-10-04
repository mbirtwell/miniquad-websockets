use crate::error::Error;
use crate::WebSocketSink;
use std::fmt::{self, Debug, Formatter};

pub struct WebSocketEvent<WebSocketId> {
    pub id: WebSocketId,
    pub kind: WebSocketEventKind,
}

#[derive(Debug)]
pub struct CloseFrame {
    code: u32,
    reason: String,
}

pub enum WebSocketEventKind {
    Connected(WebSocketSink),
    ConnectionFailed(Error),
    Message(String),
    CloseMessage(Option<CloseFrame>),
    ConnectionClosed,
    Error(Error),
}

impl Debug for WebSocketEventKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            WebSocketEventKind::Connected(_) => write!(f, "WebSocketEventKind::Connected(...)"),
            WebSocketEventKind::ConnectionFailed(err) => {
                write!(f, "WebSocketEventKind::ConnectionFailed({:?})", err)
            }
            WebSocketEventKind::Message(msg) => write!(f, "WebSocketEventKind::Message({:?})", msg),
            WebSocketEventKind::CloseMessage(frame) => {
                write!(f, "WebSocketEventKind::CloseMessage({:?})", frame,)
            }
            WebSocketEventKind::ConnectionClosed => {
                write!(f, "WebSocketEventKind::ConnectionClosed")
            }
            WebSocketEventKind::Error(err) => write!(f, "WebSocketEventKind::Error({:?})", err),
        }
    }
}

impl<WebSocketId> WebSocketEvent<WebSocketId> {
    pub fn connected(id: WebSocketId, sink: WebSocketSink) -> Self {
        Self {
            id,
            kind: WebSocketEventKind::Connected(sink),
        }
    }

    pub fn connection_failed(id: WebSocketId, err: Error) -> Self {
        Self {
            id,
            kind: WebSocketEventKind::ConnectionFailed(err),
        }
    }

    pub fn message(id: WebSocketId, msg: String) -> Self {
        Self {
            id,
            kind: WebSocketEventKind::Message(msg),
        }
    }

    pub fn empty_close_msg(id: WebSocketId) -> Self {
        Self {
            id,
            kind: WebSocketEventKind::CloseMessage(None),
        }
    }

    pub fn close_msg(id: WebSocketId, code: u32, reason: String) -> Self {
        Self {
            id,
            kind: WebSocketEventKind::CloseMessage(Some(CloseFrame { code, reason })),
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
