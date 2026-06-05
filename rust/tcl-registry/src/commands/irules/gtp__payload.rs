//! `GTP::payload` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the entire payload for G-PDU message.",
            synopsis: &["GTP::payload", "GTP::payload COUNT", "GTP::payload OFFSET COUNT", "GTP::payload 'replace' ('-message' MESSAGE)? OFFSET COUNT NEW_VALUE"],
            snippet: "Returns the payload, either complete or partial, for G-PDU message.\nThis command returns an empty value, in case of non-G-PDU messages.",
            source: "https://clouddocs.f5.com/api/irules/GTP__payload.html",
            examples: "when CLIENT_ACCEPTED {\n    set payload [UDP::payload]\n    set t2 [GTP::parse $payload]\n    log local0. \"GTP version [GTP::header version -message $t2]\"\n    log local0. \"GTP type [GTP::header type -message $t2]\"\n    log local0. \"GTP teid [GTP::header teid -message $t2]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "GTP::payload" },
        ],
        options: &[
            OptionSpec { name: "-message", takes_value: true, value_hint: "", detail: "Option -message.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
