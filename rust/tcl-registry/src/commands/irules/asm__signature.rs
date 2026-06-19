//! `ASM::signature` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::signature",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the list of signatures.",
            synopsis: &[
                "ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)",
            ],
            snippet: "Returns the list of signatures.",
            source: "https://clouddocs.f5.com/api/irules/ASM__signature.html",
            examples: "when ASM_REQUEST_DONE {\n    log local0. \"ids=[ASM::signature ids] names=[ASM::signature names] set_names=[ASM::signature set_names]\"\n    log local0. \"staged_ids=[ASM::signature staged_ids] staged_names=[ASM::signature staged_names] staged_set_names=[ASM::signature staged_set_names]\"\n}",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        ..CommandSpec::DEFAULT
    }
}
