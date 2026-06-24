//! `use` — expand a `template` inline (`use NAME ?param=value …?`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "use",
        dialects: Some(DialectSet::BPF),
        // `use NAME` plus zero or more `key=value` bindings.
        arity: Arity::at_least(1),
        ..CommandSpec::DEFAULT
    }
}
