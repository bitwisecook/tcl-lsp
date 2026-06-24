//! `attach` — declare a program's deployment target (`attach KIND TARGET`,
//! e.g. `attach xdp eth0`). Part of the profile-based top layer's
//! attach/deployment facet.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "attach",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(2),
        ..CommandSpec::DEFAULT
    }
}
