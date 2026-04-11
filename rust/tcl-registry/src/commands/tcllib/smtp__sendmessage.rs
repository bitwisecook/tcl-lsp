//! `smtp::sendmessage` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "smtp::sendmessage",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief("Send an e-mail message via SMTP.", &["smtp::sendmessage token ?-servers list? ?-ports list? ?-username user? ?-password pass? ?-usetls boo"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
