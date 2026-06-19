//! `STATS::incr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STATS::incr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Increments the value of a Statistics profile setting.",
            synopsis: &["STATS::incr PROFILE_NAME FIELD_NAME (VALUE)?"],
            snippet: "Increments the value of the specified setting (field), in the specified\nStatistics profile, by the specified value. If you do not specify a\nvalue, the system increments by 1. It is possible to set a negative\nvalue in order to decrement the counter. Returns the current value of\nthe field which was incremented.",
            source: "https://clouddocs.f5.com/api/irules/STATS__incr.html",
            examples: "when HTTP_REQUEST {\n\n   # Increment the number of unanswered HTTP requests\n   log local0. \"Incremented the current count to: [STATS::incr my_stats_profile_name \"current_count\"]\"\n}",
            return_value: "Returns the current value of the field which was incremented.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "STATS::incr PROFILE_NAME FIELD_NAME (VALUE)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IStats,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
