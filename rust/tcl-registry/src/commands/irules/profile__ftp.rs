//! `PROFILE::ftp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::ftp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the value of an FTP profile setting.",
            synopsis: &["PROFILE::ftp ATTR"],
            snippet:
                "Returns the current value of the specified setting in the assigned FTP profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__ftp.html",
            examples: "",
            return_value:
                "Returns the current value of the specified setting in the assigned FTP profile.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "PROFILE::ftp ATTR",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::BigipConfig,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}
