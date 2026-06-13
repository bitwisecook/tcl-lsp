//! `ANTIFRAUD::geo` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ANTIFRAUD::geo",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns L3 geoIP and geolocation collected by client.",
            synopsis: &["ANTIFRAUD::geo"],
            snippet: "Returns L3 geoIP and geolocation collected by client.",
            source: "https://clouddocs.f5.com/api/irules/ANTIFRAUD__geo.html",
            examples: "when ANTIFRAUD_ALERT {\n                log local0. \"geolocation: [ANTIFRAUD::geo].\"\n            }",
            return_value: "Returns L3 geoIP and geolocation collected by client.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ANTIFRAUD::geo" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
