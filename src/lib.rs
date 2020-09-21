#![feature(async_closure)]

pub use imp::{init, Message, WebSocketContext, WebSocketSink};

pub use crate::error::Result;
#[cfg(not(target_arch = "wasm32"))]
use crate::native_imp as imp;
pub use crate::request::IntoClientRequest;
#[cfg(target_arch = "wasm32")]
use crate::wasm_imp as imp;

mod error;
mod event;
mod request;

#[cfg(not(target_arch = "wasm32"))]
mod native_imp;

#[cfg(target_arch = "wasm32")]
mod wasm_imp;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
