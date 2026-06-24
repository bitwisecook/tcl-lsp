//! `field` — declare a header field inside a `profile` body
//! (`field NAME OFFSET WIDTHBITS`).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "field",
        dialects: Some(DialectSet::BPF),
        arity: Arity::exact(3),
        ..CommandSpec::DEFAULT
    }
}
