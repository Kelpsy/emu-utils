#![feature(generic_const_exprs)]
#![allow(incomplete_features)]
#![warn(clippy::all)]

extern crate alloc;
pub extern crate cfg_if;

mod bounded;
mod mem;
pub use mem::*;
mod fifo;
pub use fifo::Fifo;
pub mod schedule;
