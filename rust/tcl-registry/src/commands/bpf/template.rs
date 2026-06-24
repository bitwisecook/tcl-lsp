//! `template` — declare a reusable parameterised handler
//! (`template NAME { params } { body }`), expanded at each `use` site.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "template",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(3),
        ..CommandSpec::DEFAULT
    }
}
