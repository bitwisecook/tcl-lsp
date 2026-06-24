//! `profile` — select a built-in profile or declare one (`profile NAME { … }`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "profile",
        dialects: Some(DialectSet::BPF),
        // `profile NAME`  or  `profile NAME { body }`
        arity: Arity::new(1, 2),
        ..CommandSpec::DEFAULT
    }
}
