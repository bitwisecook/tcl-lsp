//! `allow` — restrict a program to a set of gated verbs (`allow CMD ?CMD …?`).
//! Part of the profile-based top layer's capability/policy facet.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "allow",
        dialects: Some(DialectSet::BPF),
        arity: Arity::at_least(1),
        ..CommandSpec::DEFAULT
    }
}
