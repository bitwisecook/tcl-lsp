//! `XLAT::src_config` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::src_config",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Retrieve the source-translation configuration.",
            synopsis: &["XLAT::src_config"],
            snippet: "Return the source translation configuration as a list. With the values in the following order: type,source translation object/pool.\n\ntype - The source translation type as a string. Possible values are: NONE, AUTOMAP, SNAT, LSN, SECURITY-DYNAMIC-PAT, SECURITY-DYNAMIC-NAT, SECURITY-STATIC-NAT, SECURITY-STATIC-PAT\npool - the source translation object/pool name. NA when not applicable(NONE and AUTOMAP types).",
            source: "https://clouddocs.f5.com/api/irules/XLAT__src_config.html",
            examples: "when SA_PICKED {\n    log local0. \"[XLAT::src_config]\"\n}",
            return_value: "Return the source translation configuration as a list. On error an exception is thrown with a message indicating the cause of failure.",
        }),
        excluded_events: &["RULE_INIT"],
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "XLAT::src_config" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::LsnState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
