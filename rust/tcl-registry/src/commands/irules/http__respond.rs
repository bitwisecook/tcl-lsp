//! `HTTP::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::respond",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(1),
        options: &[
            OptionSpec {
                name: "-version",
                takes_value: true,
                value_hint: "1.0 | 1.1",
                detail: "Protocol version on the synthesised response.",
                dialects: None,
            },
            OptionSpec {
                name: "-status",
                takes_value: true,
                value_hint: "reason",
                detail: "Override the default reason phrase for the status code.",
                dialects: None,
            },
            OptionSpec {
                name: "-noserver",
                takes_value: false,
                value_hint: "",
                detail: "Suppress the auto-injected `Server` response header.",
                dialects: None,
            },
        ],
        hover: Some(HoverSnippet::brief(
            "Send an immediate HTTP response from an iRule.",
            &["HTTP::respond <status> ?option value ...?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
