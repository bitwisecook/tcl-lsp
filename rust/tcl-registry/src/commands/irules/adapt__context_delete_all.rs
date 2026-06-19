//! `ADAPT::context_delete_all` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::context_delete_all",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Deletes all dynamic contexts.",
            synopsis: &["ADAPT::context_delete_all"],
            snippet: "Deletes all dynamic contexts on both sides of the virtual\nserver, making the static context the current context. This\nis done automatically when the last of a connection flow and\nits peer is torn down, so normally need not be called.\n\nSyntax:\n\nADAPT::context_delete_all",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__context_delete_all.html",
            examples: "# Conditionally revert to static contexts after request processed\n# (contrived example, probably not useful).\nwhen HTTP_PROXY_REQUEST {\n    if {$revert_to_profile} {\n        ADAPT::context_delete_all\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ADAPT::context_delete_all",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::IcapState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
