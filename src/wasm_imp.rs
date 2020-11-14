use std::borrow::Cow;
use std::ffi::{c_void, CString};

use miniquad::CustomEventPostBox;

use crate::error::{Error, Result};
use crate::WebSocketEvent;

pub struct WebSocketContext<EventType> {
    post_box: CustomEventPostBox<EventType>,
}
pub struct WebSocketSink {
    inner_id: u32,
}

pub fn init<WebSocketId, EventType>(
    post_box: CustomEventPostBox<EventType>,
) -> Result<WebSocketContext<EventType>>
where
    EventType: Send + From<WebSocketEvent<WebSocketId>>,
    WebSocketId: Clone,
{
    Ok(WebSocketContext { post_box })
}

struct WebSocket {
    id: *mut c_void,
    post_box: *mut c_void,
}

struct ConnectingCbs {
    data: WebSocket,
    connected: unsafe fn(data: WebSocket, inner_id: u32),
    connection_failed: unsafe fn(data: WebSocket),
}

extern "C" {
    fn start_connect(cb_data_ptr: *const c_void, url_ptr: *const i8, url_len: u32);
}

#[no_mangle]
pub unsafe extern "C" fn connected(data: *mut c_void, inner_id: u32) {
    let ConnectingCbs {
        data, connected, ..
    } = *Box::from_raw(data as *mut ConnectingCbs);
    connected(data, inner_id);
}

unsafe fn connected_<WebSocketId, EventType>(data: WebSocket, inner_id: u32)
where
    EventType: Send + From<WebSocketEvent<WebSocketId>> + 'static,
    WebSocketId: Send + Clone + 'static,
{
    let id = (*(data.id as *mut WebSocketId)).clone();
    let post_box = &*(data.post_box as *const CustomEventPostBox<EventType>);
    post_box.post(WebSocketEvent::connected(id, WebSocketSink { inner_id }))
}

#[no_mangle]
pub unsafe extern "C" fn connection_failed(data: *mut c_void, inner_id: u32) {
    let ConnectingCbs {
        data,
        connection_failed,
        ..
    } = *Box::from_raw(data as *mut ConnectingCbs);
    connection_failed(data);
}

unsafe fn connection_failed_<WebSocketId, EventType>(data: WebSocket)
where
    EventType: Send + From<WebSocketEvent<WebSocketId>> + 'static,
    WebSocketId: Send + Clone + 'static,
{
    let id = *Box::from_raw(data.id as *mut WebSocketId);
    let post_box = Box::from_raw(data.post_box as *mut CustomEventPostBox<EventType>);
    post_box.post(WebSocketEvent::connection_failed(
        id,
        // TODO: proper error
        Error::ConnectionClosed,
    ));
}

impl<EventType> WebSocketContext<EventType> {
    pub fn start_connect<WebSocketId>(&mut self, id: WebSocketId, request: &str) -> Result<()>
    where
        EventType: Send + From<WebSocketEvent<WebSocketId>> + 'static,
        WebSocketId: Send + Clone + 'static,
    {
        let id_box = Box::new(id);
        let post_box_box = Box::new(self.post_box.clone());
        let connecting_cbs = Box::new(ConnectingCbs {
            data: WebSocket {
                id: Box::into_raw(id_box) as _,
                post_box: Box::into_raw(post_box_box) as _,
            },
            connected: connected_::<WebSocketId, EventType>,
            connection_failed: connection_failed_::<WebSocketId, EventType>,
        });
        let url_str =
            CString::new(request).map_err(|_| Error::Url(Cow::Owned(request.to_string())))?;
        unsafe {
            start_connect(
                Box::into_raw(connecting_cbs) as _,
                url_str.as_ptr(),
                url_str.as_bytes().len() as u32,
            );
        }
        Ok(())
    }
}
impl WebSocketSink {
    pub fn send(&mut self, msg: String) -> Result<()> {
        Ok(())
    }
}
