//! `ECA::domainname` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::domainname",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns NTLM authenticating user's domain name.",
            synopsis: &["ECA::domainname"],
            snippet:
                "The ECA::domainname command returns NTLM returns authenticating user's domain name",
            source: "https://clouddocs.f5.com/api/irules/ECA__domainname.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ECA::domainname",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
