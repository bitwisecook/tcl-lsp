//! `ACCESS::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command generates new respond and automatically overrides the default respond.",
            synopsis: &["ACCESS::respond STATUS_CODE (ifile | -ifile) IFILE_OBJ", "ACCESS::respond STATUS_CODE (((('content' | '-content') CONTENT)"],
            snippet: "This command generates new respond and automatically overrides the\ndefault respond. This command only can be used only once per HTTP\nrequest, and subsequent calls to this command will return an error.\n\nHTTP iRules should be used with caution after an ACCESS::respond call.\nThey may not behave as expected since ACCESS::respond creates an HTTP response.\nAs of version 13.0.0, the way that HTTP caching interacts with the\nHTTP iRule commands has changed, so inconsistencies are expected when using\nHTTP iRules after ACCESS::respond.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__respond.html",
            examples: "when ACCESS_POLICY_COMPLETED {\n    set policy_result [ACCESS::policy result]\n    switch $policy_result {\n    \"allow\" {\n    # Do nothing\n    }\n    \"deny\" {\n        ACCESS::respond 401 content \"<html><body>Error: Failure in Authentication</body></html>\" Connection Close\n    }\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ACCESS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ACCESS::respond STATUS_CODE (ifile | -ifile) IFILE_OBJ" },
        ],
        options: &[
            OptionSpec { name: "-ifile", takes_value: false, value_hint: "", detail: "Option -ifile.", dialects: None },
            OptionSpec { name: "-content", takes_value: true, value_hint: "", detail: "Option -content.", dialects: None },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ApmState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
