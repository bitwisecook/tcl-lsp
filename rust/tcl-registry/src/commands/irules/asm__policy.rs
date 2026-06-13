//! `ASM::policy` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the name of the ASM security policy that was applied for the request.",
            synopsis: &["ASM::policy"],
            snippet: "Returns the name of the ASM policy that was applied on the request. It can be used to detect which CPM rules are applied or ASM::enable commands are applied on a request.",
            source: "https://clouddocs.f5.com/api/irules/ASM__policy.html",
            examples: "when ASM_REQUEST_BLOCKING{\n    log local0. \"The request was blocked using the [ASM::policy] policy\"\n}",
            return_value: "Returns the ASM policy applied on the request or null string if ASM is disabled.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASM::policy" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
