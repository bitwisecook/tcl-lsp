//! `SSL::clientrandom` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::clientrandom",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Return the ClientRandom value from the Client hello.",
            synopsis: &["SSL::clientrandom"],
            snippet: "Return the ClientRandom value from the Client hello.",
            source: "https://clouddocs.f5.com/api/irules/SSL__clientrandom.html",
            examples: "when CLIENTSSL_HANDSHAKE {\n    log local0.info \"negotiated protocol: [SSL::clientrandom]\"\n}",
            return_value: "The ClientRandom value.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::clientrandom" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
