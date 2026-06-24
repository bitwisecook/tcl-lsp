//! `load16` — load a 16-bit value from the packet (`load16 DST SRC OFFSET`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "load16",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(3),
        ..CommandSpec::DEFAULT
    }
}
