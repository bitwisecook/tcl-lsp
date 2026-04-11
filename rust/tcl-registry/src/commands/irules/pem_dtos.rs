//! `pem_dtos` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "pem_dtos",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Queries DTOS (Device Type and OS) database.",
            &["pem_dtos 'tac' 'lookup' PEM_DTOS_MCRO"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
