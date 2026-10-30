//! The core decides; it doesn't do. No network, no database, no clock, no files.
//!
//! The empty dependency list in Cargo.toml isn't enough on its own: the standard
//! library has `std::fs`, `std::net` and `std::time`. So the core is also `no_std`
//! (only `core` and `alloc`), and `unsafe` is forbidden, which rules out calling C.
#![no_std]
#![forbid(unsafe_code)]

#[macro_use]
extern crate alloc;

use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code { Available, Sold, Redeemed, Expired }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event { Sale, Redemption, Expiry }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command { Notify(Code), Reject(Event) }

// The core: no network, no database, no clock. An event in, decisions out.
pub fn step(code: Code, event: Event) -> (Code, Vec<Command>) {
    use Code::*;
    match (code, event) {
        (Available, Event::Sale) => (Sold, vec![Command::Notify(Sold)]),
        (Sold, Event::Redemption) => (Redeemed, vec![Command::Notify(Redeemed)]),
        (Available | Sold, Event::Expiry) => (Expired, vec![]),
        (c, e) => (c, vec![Command::Reject(e)]),
    }
}
