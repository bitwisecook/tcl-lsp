//! `PROFILE::vdi` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::vdi",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of a VDI profile setting.",
            synopsis: &["PROFILE::vdi ATTR"],
            snippet: "Returns the current value of the specified setting in the assigned VDI profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__vdi.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"\\[PROFILE::vdi msrdp_ntlm_auth_name\\]:    [PROFILE::vdi msrdp_ntlm_auth_name]\"\n    log local0. \"\\[PROFILE::vdi citrix_storefront_replacement\\]:   [PROFILE::vdi citrix_storefront_replacement]\"\n}",
            return_value: "Returns the current value of the specified setting in the assigned VDI profile.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PROFILE::vdi ATTR" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::BigipConfig,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
