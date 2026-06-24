//! `pass` — the `XDP_PASS` verdict.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pass",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(0),
        ..CommandSpec::DEFAULT
    }
}
