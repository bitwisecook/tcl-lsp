//! `accept` — accept the packet, optionally a byte count (`accept ?N?`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "accept",
        dialects: Some(DialectSet::BPF),
        arity: Arity::new(0, 1),
        ..CommandSpec::DEFAULT
    }
}
