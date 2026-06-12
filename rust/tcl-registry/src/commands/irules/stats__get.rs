//! `STATS::get` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STATS::get",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Retrieves a setting value from a Statistics profile.",
            synopsis: &["STATS::get PROFILE_NAME FIELD_NAME"],
            snippet: "Retrieves the value of the specified field of the specified Statistics\nprofile.\n\nSTATS::get <profile> <field>\n\n     * Retrieves the value of the specified field of the specified\n       Statistics profile.",
            source: "https://clouddocs.f5.com/api/irules/STATS__get.html",
            examples: "when HTTP_REQUEST {\n  if {[string tolower [HTTP::uri]] starts_with \"/check\"} {\n    STATS::get stats_profile_1 \"my_first_field\"\n  }\n}",
            return_value: "Returns the value of the specified field of the specified Statistics profile",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "STATS::get PROFILE_NAME FIELD_NAME" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::IStats,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
