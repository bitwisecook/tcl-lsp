//! `SIPALG::hairpin_default` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIPALG::hairpin_default",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets the value of hairpin flag for the current connection.",
            synopsis: &[
                "SIPALG::hairpin_default",
                "SIPALG::hairpin_default (detect | disable | enable)",
            ],
            snippet: "Returns the value of the hairpin flag for the current connection.",
            source: "https://clouddocs.f5.com/api/irules/SIPALG__hairpin_default.html",
            examples: "when SIP_REQUEST {\n    log local0. \"default hairpin mode [SIPALG::hairpin_default]\"\n}",
            return_value: "Returns 'detect', 'disable', or 'enable'",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SIPALG::hairpin_default",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
