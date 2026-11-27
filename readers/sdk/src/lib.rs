#[cfg(not(target_arch = "wasm32"))]
pub mod build;
mod dispatch;
mod files;
mod host;
mod log;
mod reader;

pub use dispatch::dispatch;
#[cfg(target_arch = "wasm32")]
pub use dispatch::serve;
pub use files::{read_file, read_text, search};
#[cfg(not(target_arch = "wasm32"))]
pub use host::{Folder, with_files};
pub use log::{Level, log, now};
pub use reader::Reader;
pub use sternegit_reader_protocol as protocol;
pub use sternegit_reader_protocol::{
    Edit, Failure, Field, Fix, FixInput, FixOffer, Found, Icon, Model, Object, Part, Place, ReadInput, Reference, Role, Scope,
    Search, Side, SideFilesInput, Value, Warning, digest,
};

pub const SDK_VERSION: &str = env!("CARGO_PKG_VERSION");

#[macro_export]
macro_rules! export {
    ($reader:expr) => {
        #[cfg(target_arch = "wasm32")]
        #[unsafe(no_mangle)]
        pub extern "C" fn sternegit_call(input_len: i32) -> i32 {
            $crate::serve(&$reader, input_len)
        }

        #[cfg(not(target_arch = "wasm32"))]
        const _: fn() = || {
            let _: &dyn $crate::Reader = &$reader;
        };
    };
}
