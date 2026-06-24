//! `pktlen` — read the packet length (`pktlen DST SRC`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pktlen",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(2),
        ..CommandSpec::DEFAULT
    }
}
