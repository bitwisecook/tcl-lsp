//! `seti32` — assign a 32-bit signed integer from an expression.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "seti32",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(2),
        ..CommandSpec::DEFAULT
    }
}
