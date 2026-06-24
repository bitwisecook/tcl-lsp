//! `tx` — the `XDP_TX` verdict (bounce the packet back out).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tx",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(0),
        ..CommandSpec::DEFAULT
    }
}
