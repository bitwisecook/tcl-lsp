//! `ONECONNECT::label` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::label",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Associate OneConnect keying information with connection.",
            synopsis: &["ONECONNECT::label update KEY"],
            snippet: "Associate OneConnect keying information with connection",
            source: "https://clouddocs.f5.com/api/irules/ONECONNECT__label.html",
            examples: "when HTTP_REQUEST {\n   set keymatch [HTTP::uri]\n   persist uie $keymatch\n   ONECONNECT::select persist\n }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ONECONNECT::label update KEY" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
