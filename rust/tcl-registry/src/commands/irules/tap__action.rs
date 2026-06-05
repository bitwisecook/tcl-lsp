//! `TAP::action` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TAP::action",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or updates security token action.",
            synopsis: &["TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?"],
            snippet: "Returns action supplied by TAP service. If supplied new action to set function returns previous action.",
            source: "https://clouddocs.f5.com/api/irules/TAP__action.html",
            examples: "when TAP_REQUEST {\n    if {    ([TAP::action] eq \"block\") } {\n        drop\n    }\n}",
            return_value: "Returns one of the following actions: allow, alarm, basicPolicy, strictPolicy, jsInection, captcha, block, tcpReset, deception.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["TAP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TAP::action (allow | alarm | basicPolicy | strictPolicy | jsInjection | captcha | block | tcpReset | deception | conviction)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
