//! `map_set` — store a value into a map (`map_set NAME {KEY} {VAL}`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "map_set",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(3),
        ..CommandSpec::DEFAULT
    }
}
