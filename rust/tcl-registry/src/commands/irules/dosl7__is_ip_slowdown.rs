//! `DOSL7::is_ip_slowdown` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::is_ip_slowdown",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns TRUE if source IP exists in greylist table",
            synopsis: &["DOSL7::is_ip_slowdown"],
            snippet: "Returns TRUE if source IP exists in greylist table",
            source: "https://clouddocs.f5.com/api/irules/DOSL7__is_ip_slowdown.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DOSL7::is_ip_slowdown",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Dosl7State,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
