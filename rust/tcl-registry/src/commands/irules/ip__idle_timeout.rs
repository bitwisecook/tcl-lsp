//! `IP::idle_timeout` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::idle_timeout",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets the idle timeout value.",
            synopsis: &["IP::idle_timeout (TIMEOUT)?"],
            snippet: "Returns the idle timeout value, or specifies an idle timeout value as the criteria for selecting the pool to which you want the BIG-IP system to send traffic.",
            source: "https://clouddocs.f5.com/api/irules/IP__idle_timeout.html",
            examples: "when SERVER_CONNECTED {\n    IP::idle_timeout $idle\n}",
            return_value: "idle timeout value in seconds",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "IP::idle_timeout (TIMEOUT)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
