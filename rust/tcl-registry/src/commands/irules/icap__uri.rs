//! `ICAP::uri` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ICAP::uri",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets or returns the ICAP request URI.",
            synopsis: &["ICAP::uri (URI_STRING)?"],
            snippet: "The ICAP::uri command sets or returns the ICAP request URI.",
            source: "https://clouddocs.f5.com/api/irules/ICAP__uri.html",
            examples: "when ICAP_REQUEST {\n                if {[ICAP::uri] contains \"movie\"} {\n                    ICAP::uri http://icap.mydomain.org/video\n                }\n            }",
            return_value: "Returns the ICAP request URI.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ICAP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ICAP::uri (URI_STRING)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::IcapState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}
