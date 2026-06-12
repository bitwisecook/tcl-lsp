//! `GTP::parse` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::parse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Creates a new GTP message from a byte stream.",
            synopsis: &["GTP::parse BYTE_STREAM"],
            snippet: "Creates a new GTP message from a byte stream.\nReturns a TCL object of type \"GTP-Message\"",
            source: "https://clouddocs.f5.com/api/irules/GTP__parse.html",
            examples: "when CLIENT_ACCEPTED {\n    set payload [UDP::payload]\n    set t2 [GTP::parse $payload]\n    log local0. \"GTP type [GTP::header type -message $t2]\"\n    log local0. \"GTP teid [GTP::header teid -message $t2]\"\n}",
            return_value: "Returns a TCL object of type \"GTP-Message\"",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "GTP::parse BYTE_STREAM" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
