#![warn(clippy::all)]

extern crate alloc;
pub extern crate static_assertions;

pub use emu_utils_macros::*;
pub mod bitfield;
mod bounded;
mod mem;
pub use mem::*;
mod fifo;
pub use fifo::Fifo;
