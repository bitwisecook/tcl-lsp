//! `ntohs` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ntohs",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary:
                "Converts the unsigned short integer from network byte order to host byte order.",
            synopsis: &["ntohs NUMBER"],
            snippet:
                "Convert the unsigned short integer from network byte order to host byte\norder.",
            source: "https://clouddocs.f5.com/api/irules/ntohs.html",
            examples:
                "when HTTP_REQUEST {\n  set netshort 1234\n  set hostshort [ntohs $netshort]\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ntohs NUMBER",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
