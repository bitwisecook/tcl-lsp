//! `ADAPT::select` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets or returns the internal virtual server (IVS) selection.",
            synopsis: &["ADAPT::select (ADAPT_CTX)? (ADAPT_SIDE)? (NAME)?"],
            snippet: "The ADAPT::select command returns or selects the name of\nthe internal virtual server (IVS) associated with the ADAPT\nfilter on the current or specified side of the virtual server\nconnection for which the iRule is being executed.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__select.html",
            examples: "when HTTP_RESPONSE {\n     if { [HTTP::header \"Content-Type\"] contains \"image\" } {\n        ADAPT::select ivs-icap-image\n        ADAPT::preview_size 10000\n        ADAPT::enable yes\n     }\n     if { [HTTP::header \"Content-Type\"] contains \"video\" } {\n        ADAPT::select ivs-icap-video\n        ADAPT::preview_size 30000\n        ADAPT::enable yes\n     }\n}",
            return_value: "Returns the current or new internal virtual server (IVS) name.",
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
            FormSpec { kind: FormKind::Default, synopsis: "ADAPT::select (ADAPT_CTX)? (ADAPT_SIDE)? (NAME)?" },
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
