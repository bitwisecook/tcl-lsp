//! `GTP::new` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::new",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Creates a new GTP message for given version & request-type.",
            synopsis: &["GTP::new VERSION TYPE"],
            snippet: "Creates a new GTP message for given version & request-type.\nValid values for version are 1 or 2 only.\nRequest-type: A value less than 256.\nReturns a TCL object of type \"GTP-Message\"",
            source: "https://clouddocs.f5.com/api/irules/GTP__new.html",
            examples: "when CLIENT_ACCEPTED {\n    set t2 [GTP::new 2 10]\n    log local0. \"GTP version [GTP::header version -message $t2]\"\n    log local0. \"GTP type [GTP::header type -message $t2]\"\n}",
            return_value: "Returns a TCL object of type \"GTP-Message\"",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "GTP::new VERSION TYPE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
