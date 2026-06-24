//! `map_get` — read a value from a map, 0 if absent (`map_get DST NAME {KEY}`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "map_get",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(3),
        ..CommandSpec::DEFAULT
    }
}
