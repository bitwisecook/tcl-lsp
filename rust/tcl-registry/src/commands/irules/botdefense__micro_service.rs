//! `BOTDEFENSE::micro_service` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BOTDEFENSE::micro_service",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the micro-service that matched the current request.",
            synopsis: &["BOTDEFENSE::micro_service (name | type)"],
            snippet: "Returns the micro-service that matched the current request.",
            source: "https://clouddocs.f5.com/api/irules/BOTDEFENSE__micro_service.html",
            examples: "when BOTDEFENSE_ACTION {\n    set ms [BOTDEFENSE::micro_service name]\n    if { $ms neq \"\"} {\n        log.local0. \"Request to micro_service $ms of type [BOTDEFENSE::micro_service type]\n    }\n}",
            return_value: "Returns the name or type of the micro-service found for the current request",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["BOTDEFENSE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BOTDEFENSE::micro_service (name | type)" },
        ],
        ..CommandSpec::DEFAULT
    }
}
