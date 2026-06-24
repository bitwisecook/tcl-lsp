//! `setbuf` — bind a packet/context buffer pointer (`setbuf NAME ctx`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "setbuf",
        dialects: Some(DialectSet::BPF),
        // `setbuf NAME ctx` or `setbuf NAME = ctx`.
        arity: Arity::new(2, 3),
        ..CommandSpec::DEFAULT
    }
}
