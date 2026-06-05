//! `PSC::calling_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::calling_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set calling id.",
            synopsis: &["PSC::calling_id (CALLING_ID)?"],
            snippet: "The PSC::calling_id command gets the calling station id or sets the\ncalling station id when the optional value is given.",
            source: "https://clouddocs.f5.com/api/irules/PSC__calling_id.html",
            examples: "",
            return_value: "Return the calling station id when no argument is given.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PSC::calling_id (CALLING_ID)?" },
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
