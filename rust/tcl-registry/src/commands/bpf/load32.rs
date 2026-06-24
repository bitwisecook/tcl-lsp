//! `load32` — load a 32-bit value from the packet (`load32 DST SRC OFFSET`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "load32",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(3),
        ..CommandSpec::DEFAULT
    }
}
