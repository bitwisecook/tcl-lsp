//! `HTTP::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Changes the HTTP filter from passthrough to full parsing mode.",
            synopsis: &["HTTP::enable"],
            snippet: "Changes the HTTP filter from passthrough to full parsing mode. This\ncould be useful, for instance, if you need to determine whether or not\nHTTP is passing over the connection and enable the HTTP filter\nappropriately, or if you have a protocol that is almost but not quite\nlike HTTP, and you need to re-enable HTTP parsing after temporarily\ndisabling it.\nUse of this command can be extremely tricky to get exactly right; its\nuse is not recommended in the majority of cases.\nNote: This command does not function in certain versions of BIG-IP\n(v9.4.0 - v9.4.4).",
            source: "https://clouddocs.f5.com/api/irules/HTTP__enable.html",
            examples: "when HTTP_REQUEST {\nlog local0. \"Got request: [HTTP::uri]\"\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
