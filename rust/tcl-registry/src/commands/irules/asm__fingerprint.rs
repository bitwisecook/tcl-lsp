//! `ASM::fingerprint` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::fingerprint",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the fingerprint (device id) of the client device.",
            synopsis: &["ASM::fingerprint"],
            snippet: "Get the fingerprint of the client device as seen by ASM when it's available.\nThe fingerprint is a unique identifier given to specific client machine. The fingerprint will be available to iRule only for web application that have web scraping turned on with the finger print usage activated.",
            source: "https://clouddocs.f5.com/api/irules/ASM__fingerprint.html",
            examples: "when ASM_REQUEST_DONE {\n    log local0.[ASM::fingerprint]\n}",
            return_value: "Returns the fingerprint of the client device or 0 if it's not available.",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::fingerprint",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        ..CommandSpec::DEFAULT
    }
}
