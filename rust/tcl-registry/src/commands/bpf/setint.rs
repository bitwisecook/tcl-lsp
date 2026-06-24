//! `setint` — assign a 64-bit integer from an expression.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "setint",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(2),
        ..CommandSpec::DEFAULT
    }
}
