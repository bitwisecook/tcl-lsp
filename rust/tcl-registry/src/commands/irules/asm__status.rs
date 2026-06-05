//! `ASM::status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the current status of the request or response.",
            synopsis: &["ASM::status"],
            snippet: "Returns the current status of the request or response\nReturns one of the following values:\n  + Alarm - there are violations and alarm has been raised, but\n    request or response is not blocked. This does not apply to\n    violations that are in staging mode. This value will also be\n    returned if the request had violations but was unblocked using\n    a previously called ASM::unblock command.\n  + Blocked - violations caused the request/response to be\n    blocked. This does not apply to violations that are in staging\n    mode.\n  + Clear - no violations found",
            source: "https://clouddocs.f5.com/api/irules/ASM__status.html",
            examples: "when ASM_REQUEST_DONE {\n    #log local0.debug \"\\[ASM::status\\] = [ASM::status]\"\n    if { [ASM::status] equals \"alarmed\" } {\n        set x [ASM::violation_data]\n        HTTP::header insert X-ASM \"violation=[lindex $x 0] supportid=[lindex $x 1]\"\n        #log local0.debug \"DEBUG02: violation=[lindex $x 0] supportid=[lindex $x 1]\"\n    }\n}",
            return_value: "* Alarm * Blocked * Clear",
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
