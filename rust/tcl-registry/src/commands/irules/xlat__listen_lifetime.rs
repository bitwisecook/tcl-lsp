//! `XLAT::listen_lifetime` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::listen_lifetime",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set/Get the listener lifetime.",
            synopsis: &["XLAT::listen_lifetime (HANDLE)+ (XLAT_LIFETIME)?"],
            snippet: "Set/Get the listener lifetime.\nValid range is between 0 and 31536000 (365 days).",
            source: "https://clouddocs.f5.com/api/irules/XLAT__listen_lifetime.html",
            examples: "when SERVER_CONNECTED {\n    set listener [XLAT::listen 30 {\n        proto [IP::protocol]\n        bind -allow [serverside {LINK::vlan_id}] -ip [serverside {IP::local_addr}]\n        server [IP::client_addr] [expr [TCP::local_port] + 1]\n        allow [LB::server addr] 0\n    }]\n    log local0. \"[XLAT::listen_lifetime $listener]\"\n}",
            return_value: "Return the listener lifetime value.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "XLAT::listen_lifetime (HANDLE)+ (XLAT_LIFETIME)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::LsnState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
