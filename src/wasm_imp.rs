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

extern "C" {
    fn websocket_start_connect(cb_data_ptr: *const c_void, url_ptr: *const i8, url_len: u32);
    fn websocket_send(inner_id: u32, msg_ptr: *const i8, msg_len: u32);
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

#[derive(Clone)]
struct WebSocket {
    id: *mut c_void,
    post_box: *mut c_void,
}

struct ConnectingCbs {
    data: WebSocket,
    on_open: unsafe fn(data: WebSocket, inner_id: u32) -> Box<RunningCbs>,
    on_connection_failed: unsafe fn(data: WebSocket),
}

#[no_mangle]
pub unsafe extern "C" fn on_open(data: *mut c_void, inner_id: u32) -> *mut c_void {
    let ConnectingCbs { data, on_open, .. } = *Box::from_raw(data as *mut ConnectingCbs);
    let running_cbs = on_open(data, inner_id);
    Box::into_raw(running_cbs) as *mut _
}

unsafe fn on_open_<WebSocketId, EventType>(data: WebSocket, inner_id: u32) -> Box<RunningCbs>
where
    EventType: Send + From<WebSocketEvent<WebSocketId>> + 'static,
    WebSocketId: Send + Clone + 'static,
{
    let id = (*(data.id as *mut WebSocketId)).clone();
    let post_box = &*(data.post_box as *const CustomEventPostBox<EventType>);
    post_box.post(WebSocketEvent::connected(id, WebSocketSink { inner_id }));
    Box::new(RunningCbs {
        data,
        on_message: on_message_::<WebSocketId, EventType>,
        on_close: on_close_::<WebSocketId, EventType>,
        on_error: on_error_::<WebSocketId, EventType>,
    })
}

#[no_mangle]
pub unsafe extern "C" fn on_connection_failed(data: *mut c_void) {
    let ConnectingCbs {
        data,
        on_connection_failed,
        ..
    } = *Box::from_raw(data as *mut ConnectingCbs);
    on_connection_failed(data);
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

struct RunningCbs {
    data: WebSocket,
    // TODO example passing data:
    //  native/sapp-wasm/js/gl.js:1110
    //  native/sapp-wasm/src/lib.rs:356
    on_message: unsafe fn(data: &mut WebSocket, msg_ptr: *mut u8, msg_len: usize),
    on_close: unsafe fn(
        data: &mut WebSocket,
        code: u32,
        reason_ptr: *mut u8,
        reason_len: usize,
        was_clean: bool,
    ),
    on_error: unsafe fn(data: &mut WebSocket),
}

#[no_mangle]
pub unsafe extern "C" fn on_message(data: *mut c_void, msg_ptr: *mut u8, msg_len: usize) {
    let cbs = &mut *(data as *mut RunningCbs);
    (cbs.on_message)(&mut cbs.data, msg_ptr, msg_len);
}

unsafe fn on_message_<WebSocketId, EventType>(
    data: &mut WebSocket,
    msg_ptr: *mut u8,
    msg_len: usize,
) where
    EventType: Send + From<WebSocketEvent<WebSocketId>> + 'static,
    WebSocketId: Send + Clone + 'static,
{
    let id = (*(data.id as *mut WebSocketId)).clone();
    let post_box = &*(data.post_box as *const CustomEventPostBox<EventType>);
    let msg = String::from_raw_parts(msg_ptr, msg_len, msg_len);
    post_box.post(WebSocketEvent::message(id, msg));
}

#[no_mangle]
pub unsafe extern "C" fn on_close(
    data: *mut c_void,
    code: u32,
    reason_ptr: *mut u8,
    reason_len: usize,
    was_clean: bool,
) {
    let cbs = &mut *(data as *mut RunningCbs);
    (cbs.on_close)(&mut cbs.data, code, reason_ptr, reason_len, was_clean);
}

unsafe fn on_close_<WebSocketId, EventType>(
    data: &mut WebSocket,
    code: u32,
    reason_ptr: *mut u8,
    reason_len: usize,
    _was_clean: bool,
) where
    EventType: Send + From<WebSocketEvent<WebSocketId>> + 'static,
    WebSocketId: Send + Clone + 'static,
{
    let id = (*(data.id as *mut WebSocketId)).clone();
    let post_box = &*(data.post_box as *const CustomEventPostBox<EventType>);
    let reason = String::from_raw_parts(reason_ptr, reason_len, reason_len);
    post_box.post(WebSocketEvent::close_msg(id, code, reason));
}

#[no_mangle]
pub unsafe extern "C" fn on_error(data: *mut c_void) {}

unsafe fn on_error_<WebSocketId, EventType>(data: &mut WebSocket)
where
    EventType: Send + From<WebSocketEvent<WebSocketId>> + 'static,
    WebSocketId: Send + Clone + 'static,
{
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
            on_open: on_open_::<WebSocketId, EventType>,
            on_connection_failed: connection_failed_::<WebSocketId, EventType>,
        });
        let url_str =
            CString::new(request).map_err(|_| Error::Url(Cow::Owned(request.to_string())))?;
        unsafe {
            websocket_start_connect(
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
        // TODO Proper error
        let msg_str = CString::new(msg).map_err(|_| Error::UnsupportedDataFrame)?;
        unsafe {
            websocket_send(
                self.inner_id,
                msg_str.as_ptr(),
                msg_str.as_bytes().len() as u32,
            );
        }
        Ok(())
    }
}
