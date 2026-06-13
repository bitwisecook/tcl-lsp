//! Placeholder for commands disabled in the iRules dialect.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "disabled_in_irules",
        ..CommandSpec::DEFAULT
    }
}
