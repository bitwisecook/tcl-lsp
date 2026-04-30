//! `HTTP::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::header",
        traits: Traits::PURE | Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Inspect or mutate HTTP headers in an iRule event.",
            &["HTTP::header <subcommand> ?arg ...?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
