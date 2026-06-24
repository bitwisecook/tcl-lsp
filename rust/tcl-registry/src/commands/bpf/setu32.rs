//! `setu32` — assign a 32-bit unsigned integer from an expression.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "setu32",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(2),
        ..CommandSpec::DEFAULT
    }
}
