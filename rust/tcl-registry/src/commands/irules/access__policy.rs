//! `ACCESS::policy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Return information about access policies.",
            synopsis: &["ACCESS::policy agent_id", "ACCESS::policy evaluate ('-sid' SESSION_ID)", "ACCESS::policy result (-sid SESSION_ID)?", "ACCESS::policy uri"],
            snippet: "The ACCESS::policy commands allow you to retrieve information about the\naccess policies in place for a given connection.\n\nACCESS::policy agent_id\n\n     * Returns the identifier for the agent raising the\n       ACCESS_POLICY_AGENT_EVENT.\n\nACCESS::policy result\n\n     * Returns back the result of an access policy. The result will be one\n       of following:\n     * - allow\n     * - deny\n     * - redirect\n\nACCESS::policy uri\n\n     * Returns TRUE if current request URI is internal to ACCESS (v11+\n       only).",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__policy.html",
            examples: "when CLIENT_CLOSED {\n    # To avoid clutter, remove the access session for the flow.\n    ACCESS::session remove -sid $flow_sid\n}",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ACCESS::policy <subcommand> ?args?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
