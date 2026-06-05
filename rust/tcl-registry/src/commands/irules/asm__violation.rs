//! `ASM::violation` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::violation",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the list of violations found in the request or response together with details on each one.",
            synopsis: &["ASM::violation (count | names | attack_types | details | rating)"],
            snippet: "Returns the list of violations found in the request or response together with details on each one.",
            source: "https://clouddocs.f5.com/api/irules/ASM__violation.html",
            examples: "when ASM_REQUEST_DONE {\n  set i 0\n  foreach {viol} [ASM::violation names]{\n    if {$viol eq \"VIOLATION_ILLEGAL_PARAMETER\"} {\n      set details [lindex [ASM::violation details] $i]\n      set param_name [b64_decode [llookup $details \"param_data.param_name\"]]\n      #remove the bad parameter from the QS - does not work right in all cases,just for illustration!\n      regsub -all \"\\?.*($param_name=[^\\&]*)\" [HTTP::uri] \"?\" $new_uri\n      HTTP::uri $new_uri\n      ASM::unblock\n    }",
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
