//! `SSL::c3d` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::c3d",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Inserts a certificate extension to the C3D certificate, sets the C3D client cert",
            &["SSL::c3d extension (ARG ARG)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
