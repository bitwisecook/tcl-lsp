//! `load8` — load an 8-bit value from the packet (`load8 DST SRC OFFSET`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "load8",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(3),
        ..CommandSpec::DEFAULT
    }
}
