//! `when` — attach a handler to a BPF event (`when EVENT ?priority N? { body }`).
//!
//! Unlike the F5 iRules `when`, this spec carries **no** `LoweringHookId::When`,
//! so the BPF-Tcl front-end keeps it a generic call and re-lowers the body in
//! its own (BPF-native) event space.
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "when",
        dialects: Some(DialectSet::BPF),
        // EVENT BODY  |  EVENT priority N BODY
        arity: Arity::new(2, 4),
        ..CommandSpec::DEFAULT
    }
}
