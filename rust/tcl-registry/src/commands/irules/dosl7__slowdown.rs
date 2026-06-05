//! `DOSL7::slowdown` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DOSL7::slowdown",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Adds source IP extracted from current connection to greylist.",
            synopsis: &["DOSL7::slowdown RATE TIMEOUT"],
            snippet: "Adds source IP extracted from current connection to greylist. TCP slowdown will be applied according to supplied RATE (in percents) and TIMEOUT (in seconds).\nA RATE represents amount of incoming data packets to be dropped to perform slowdown.",
            source: "https://clouddocs.f5.com/api/irules/DOSL7__slowdown.html",
            examples: "when HTTP_REQUEST {\n                 if { [HTTP::uri] contains \"heavy.php\" } {\n                     DOSL7::slowdown 30 60\n                 }\n             }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DOSL7::slowdown RATE TIMEOUT" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Dosl7State,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
