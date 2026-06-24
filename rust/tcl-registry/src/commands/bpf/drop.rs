//! `drop` — drop the packet (verdict 0).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "drop",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(0),
        ..CommandSpec::DEFAULT
    }
}
