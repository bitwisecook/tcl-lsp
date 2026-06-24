//! `deny` — forbid a set of verbs (`deny CMD ?CMD …?`). Part of the
//! profile-based top layer's capability/policy facet.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "deny",
        dialects: Some(DialectSet::BPF),
        arity: Arity::at_least(1),
        ..CommandSpec::DEFAULT
    }
}
