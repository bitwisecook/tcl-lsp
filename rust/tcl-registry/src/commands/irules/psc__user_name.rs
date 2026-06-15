//! `PSC::user_name` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::user_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set user name.",
            synopsis: &["PSC::user_name (USERNAME)?"],
            snippet: "The PSC::user_name command gets the user name or sets the user name when the optional value is given.",
            source: "https://clouddocs.f5.com/api/irules/PSC__user_name.html",
            examples: "",
            return_value: "Return the user name when no argument is given.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PSC::user_name (USERNAME)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
