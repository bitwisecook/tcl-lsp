//! `GTP::ie` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::ie",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This set of commands allows for the parsing and interpretation of GTP IE elements.",
            synopsis: &["GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?", "GTP::ie 'count' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?", "GTP::ie 'get' ('instance' | 'length' | 'encode-type' | 'value') ('-message' MESSAGE)? IE_PATH", "GTP::ie 'get' 'list' ('-message' MESSAGE)? ('-type' TYPE)? ('-instance' INSTANCE)? (IE_PATH)?"],
            snippet: "This set of commands allows for the parsing and interpretation of GTP\nIE elements.",
            source: "https://clouddocs.f5.com/api/irules/GTP__ie.html",
            examples: "when GTP_SIGNALLING_INGRESS {\n    if { [GTP::ie exists imsi:0] } {\n        log local0. \"GTP imsi [GTP::ie get value imsi:0]\"\n    }\n    log local0. \"Total number of top level IEs [GTP::ie count]\"\n    set ie_list [ GTP::ie get list]\n    foreach ie $ie_list {\n        log local0. \"IE $ie\"\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "GTP::ie 'exists' ('-message' MESSAGE)? (IE_PATH)?" },
        ],
        options: &[
            OptionSpec { name: "-message", takes_value: true, value_hint: "MESSAGE", detail: "Operate on a specific GTP message object.", dialects: None },
            OptionSpec { name: "-type", takes_value: true, value_hint: "TYPE", detail: "Filter by IE type value.", dialects: None },
            OptionSpec { name: "-instance", takes_value: true, value_hint: "INSTANCE", detail: "Filter by IE instance.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
