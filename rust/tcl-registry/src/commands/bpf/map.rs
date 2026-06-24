//! `map` — declare a BPF map (`map NAME hash KEYSZ VALSZ MAX`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "map",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(5),
        ..CommandSpec::DEFAULT
    }
}
