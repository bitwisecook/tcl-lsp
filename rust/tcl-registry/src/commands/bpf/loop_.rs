//! `loop` — a bounded, unrolled loop (`loop N VAR { body }`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "loop",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(3),
        ..CommandSpec::DEFAULT
    }
}
