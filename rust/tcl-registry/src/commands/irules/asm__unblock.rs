//! `ASM::unblock` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::unblock",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Overrides the blocking action for a request that had blocking violation.",
            synopsis: &["ASM::unblock"],
            snippet: "Overrides the blocking action for a request that had blocking\nviolations. Consequently, the request will be forwarded to the origin\nserver and also marked with a special \"unblocked\" flag which can be\nviewed in the request log. If the present request was not supposed to\nbe blocked then the command has no effect.",
            source: "https://clouddocs.f5.com/api/irules/ASM__unblock.html",
            examples: "when ASM_REQUEST_DONE {\n  set i 0\n  foreach {viol} [ASM::violation names]{\n  if {$viol eq VIOLATION_ILLEGAL_PARAMETER} {\n    set details [lindex [ASM::violation details] $i]\n    set param_name [b64decode [llookup $details \"param_data.param_name\"]]\n    #remove the bad parameter from the QS - does not work right in all cases, just for illustration!\n    regsub -all \"\\?.*($param_name=^\\&*)\" [HTTP::uri] \"?\" $new_uri\n    HTTP::uri $new_uri\n    ASM::unblock\n  }\n  set i [expr {$i+1}]\n  }\n\n}",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASM::unblock" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
