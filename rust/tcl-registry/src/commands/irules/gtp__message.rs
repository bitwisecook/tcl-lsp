//! `GTP::message` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::message",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the entire GTP message.",
            synopsis: &["GTP::message ('-message' MESSAGE)?"],
            snippet: "Returns the entire GTP message.",
            source: "https://clouddocs.f5.com/api/irules/GTP__message.html",
            examples: "when GTP_SIGNALLING_INGRESS {\n    set t1 [GTP::message]\n    log local0. \"GTP type [GTP::header type -message $t1]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "GTP::message ('-message' MESSAGE)?" },
        ],
        options: &[
            OptionSpec { name: "-message", takes_value: true, value_hint: "", detail: "Option -message.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
