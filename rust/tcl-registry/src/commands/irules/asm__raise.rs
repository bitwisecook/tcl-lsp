//! `ASM::raise` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::raise",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Issues a user-defined violation on the request.",
            synopsis: &["ASM::raise VIOLATION_NAME (VIOLATION_DETAILS)?"],
            snippet: "Issues a user-defined violation on the request. The violation\nis added to other possible violations, either raised by the ASM or by\nprevious invocations of this command. The consequent action is\ndetermined by the blocking setting per the raised violation, e.g. if\nthe violation was set to block, then the request will be blocked.",
            source: "https://clouddocs.f5.com/api/irules/ASM__raise.html",
            examples: "when ASM_REQUEST_DONE {\n   if {[ASM::violation count] > 3 and [ASM::severity] eq \"Error\"} {\n      ASM::raise VIOLATION_TOO_MANY_VIOLATIONS\n   }\n}",
            return_value: "",
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
        ..CommandSpec::DEFAULT
    }
}
