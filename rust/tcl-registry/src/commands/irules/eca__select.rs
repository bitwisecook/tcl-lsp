//! `ECA::select` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Selects one of NTLM Authentication configuration name and Kerberos Authentication configuration name or both",
            synopsis: &["ECA::select"],
            snippet: "ECA::select is to select one of NTLM Authentication configuration name and Kerberos Authentication configuration name or both used when enforcing NTLM authentication or Kerberos authentication or Kerberos NTLM fallback option respectively for this particular connection.",
            source: "https://clouddocs.f5.com/api/irules/ECA__select.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ECA::select" },
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
