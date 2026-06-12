//! `MR::transport` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::transport",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the name and type (virtual or config) of the transport used to configure the current connection.",
            synopsis: &["MR::transport"],
            snippet: "Returns the name and type (virtual or config) of the transport used to configure the current connection. These values can be used to generate routes or to set the route of a message.",
            source: "https://clouddocs.f5.com/api/irules/MR__transport.html",
            examples: "when MR_EGRESS {\n    log local0. \"sending message via [MR::transport]\"\n}",
            return_value: "Returns the name and type (virtual or config) of the transport used to configure the current connection.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::transport" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
