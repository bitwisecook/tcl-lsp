//! `GTP::clone` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::clone",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a cloned copy of the GTP message.",
            synopsis: &["GTP::clone (MESSAGE_VAR)?"],
            snippet: "Returns a cloned copy of the GTP message.",
            source: "https://clouddocs.f5.com/api/irules/GTP__clone.html",
            examples: "when CLIENT_ACCEPTED {\n    set payload [UDP::payload]\n    set t2 [GTP::parse $payload]\n    set t3 [GTP::clone $t2]\n    log local0. \"GTP type [GTP::header type -message $t3]\"\n    log local0. \"GTP teid [GTP::header teid -message $t3]\"\n}",
            return_value: "Returns a cloned copy of the GTP message.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "GTP::clone (MESSAGE_VAR)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
