//! `ACCESS::restrict_irule_events` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::restrict_irule_events",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enable or disable HTTP and higher layer iRule events for the internal APM access control URIs.",
            synopsis: &["ACCESS::restrict_irule_events (enable | disable)"],
            snippet: "During access policy execution, ACCESS creates requests to various URIs\nrelated to various access policy processing. These includes /my.policy\nand other pages (logon, message box etc.) shown to the end user. By\ndefault from 11.0.0 onward, HTTP and higher layer iRule events are not\nraised for the internal access control URIs. All events except\nACCESS_SESSION_STARTED, ACCESS_SESSION_CLOSED,\nACCESS_POLICY_AGENT_EVENT, ACCESS_POLICY_COMPLETED are blocked (not\nraised) for internal access control URI.\nThis command allows admin to overwrite the default behavior.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__restrict_irule_events.html",
            examples: "when CLIENT_ACCEPTED {\n    ACCESS::restrict_irule_events disable\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ACCESS::restrict_irule_events (enable | disable)" },
        ],
        ..CommandSpec::DEFAULT
    }
}
