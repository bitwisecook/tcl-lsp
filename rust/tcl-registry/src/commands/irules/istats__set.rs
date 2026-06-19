//! `ISTATS::set` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ISTATS::set",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the given key's value within iStats.",
            synopsis: &["ISTATS::set KEY VALUE"],
            snippet: "Set the given key's value within iStats",
            source: "https://clouddocs.f5.com/api/irules/ISTATS__set.html",
            examples: "when HTTP_REQUEST {\n  # send request to /invalidate?policy=<policy>\n  if { [HTTP::path] eq \"/invalidate\" } {\n        set wa_policy [URI::query [HTTP::uri] policy]\n        if { $wa_policy ne \"\" } {\n          ISTATS::set \"WA policy string $wa_policy\" \"invalidated\"\n        }\n        HTTP::respond 200 content \"<html><body>Cache Invalidated for Policy: $wa_policy</body></html>\"\n  }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ISTATS::set KEY VALUE",
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
