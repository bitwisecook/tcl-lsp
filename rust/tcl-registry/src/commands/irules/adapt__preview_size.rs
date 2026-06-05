//! `ADAPT::preview_size` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::preview_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets or returns the preview-size attribute.",
            synopsis: &["ADAPT::preview_size (ADAPT_CTX)? (ADAPT_SIDE)? (SIZE)?"],
            snippet: "The ADAPT::preview_size command sets or returns the preview-size\nattribute of the ADAPT filter on the current or specified side of\nthe virtual server connection for which the iRule is being executed.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__preview_size.html",
            examples: "when HTTP_RESPONSE {\n    if { [HTTP::header \"Content-Type\"] contains \"image\" } {\n        ADAPT::select ivs-icap-image\n        ADAPT::preview_size 10000\n    }\n    if { [HTTP::header \"Content-Type\"] contains \"video\" } {\n       ADAPT::select ivs-icap-video\n       ADAPT::preview_size 30000\n    }\n}",
            return_value: "Returns the current or modified preview size (bytes).",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP", "REQUESTADAPT", "RESPONSEADAPT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ADAPT::preview_size (ADAPT_CTX)? (ADAPT_SIDE)? (SIZE)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::IcapState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
