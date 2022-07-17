#![warn(clippy::all)]

mod savestate;

use proc_macro::TokenStream;

#[proc_macro_derive(Savestate, attributes(load, store, savestate))]
pub fn saveable_derive(input: TokenStream) -> TokenStream {
    savestate::derive(input)
}
