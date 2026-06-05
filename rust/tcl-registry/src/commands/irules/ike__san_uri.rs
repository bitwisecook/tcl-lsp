//! `IKE::san_uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IKE::san_uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "something",
            synopsis: &["IKE::san_uri (ANY_CHARS)*"],
            snippet: "something",
            source: "https://clouddocs.f5.com/api/irules/IKE__san_uri.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IKE::san_uri (ANY_CHARS)*",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
