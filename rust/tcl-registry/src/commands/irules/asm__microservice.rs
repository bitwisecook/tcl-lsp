//! `ASM::microservice` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::microservice",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "request matched microservice",
            synopsis: &["ASM::microservice"],
            snippet: "returns the microservice matched for the request;",
            source: "https://clouddocs.f5.com/api/irules/ASM__microservice.html",
            examples: "when ASM_REQUEST_DONE \n            {\n\t\tif {[ASM::microservice] eq \"*a/login.php\"}\n\t\t{\n\t\t\tlog local0. \"Microservice : found\"\n\t        }\n            }",
            return_value: "returns the microservice matched for the request;",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASM::microservice" },
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
