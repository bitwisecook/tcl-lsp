//! `llookup` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "llookup",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::exact(2),
hover: Some(HoverSnippet {
            summary: "Returns a list of values corresponding to the given key in a multimap.",
            synopsis: &["llookup MMAP KEY"],
            snippet: "A *multimap* is a flat Tcl list of `{key value}` pairs — the same structure returned by `[ASM::violation details]`.  Because the same key can appear more than once, `llookup` returns **a list** of every value whose key matches *KEY*.\n\nReturns an empty string when *KEY* is absent or *MMAP* is not a properly structured multimap.\n\nEquivalent Tcl (what `llookup` replaces):\n```tcl\nset r {}\nforeach pair $mmap {\n    if {[lindex $pair 0] eq $key} {\n        lappend r [lindex $pair 1]\n    }\n}\n```",
            source: "https://clouddocs.f5.com/api/irules/llookup.html",
            examples: "# Iterate violations in parallel using llookup\nwhen ASM_REQUEST_DONE {\n    set details [ASM::violation details]\n    foreach viol_name       [llookup $details viol_name] \\\n            sanity_status   [llookup $details http_sanity_checks_status] \\\n            sub_viol_status [llookup $details http_sub_violation_status] {\n        log local0.info \"$viol_name $sanity_status $sub_viol_status\"\n    }\n}",
            return_value: "A Tcl list of values matching *KEY*.  When used with `[ASM::violation details]`, binary values such as `http_sub_violation` and `sig_data.kw_data.buffer` are base64-encoded.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "llookup MMAP KEY" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
