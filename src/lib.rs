pub use imp::{init, WebSocketContext, WebSocketSink};

pub use crate::error::Result;
pub use crate::event::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::native_imp as imp;
#[cfg(target_arch = "wasm32")]
use crate::wasm_imp as imp;

mod error;
mod event;

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
