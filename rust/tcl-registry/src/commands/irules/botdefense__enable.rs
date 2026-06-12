//! `BOTDEFENSE::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables processing by Bot Defense on the connection.",
            synopsis: &["BOTDEFENSE::enable"],
            snippet: "Enables processing and blocking of the request by Bot Defense, for the duration of the current TCP connection, or until BOTDEFENSE::disable is called.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__enable.html",
            examples: "# EXAMPLE: Re-enable Bot Defense on the connection if a request arrives with a certain URL prefix.\nwhen HTTP_REQUEST {\n    if {[HTTP::uri] starts_with \"/t/\"} {\n        BOTDEFENSE::enable\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::enable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
