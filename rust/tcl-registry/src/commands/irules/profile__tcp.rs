//! `PROFILE::tcp` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PROFILE::tcp",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the value of a TCP profile setting.",
            synopsis: &["PROFILE::tcp ATTR"],
            snippet: "Returns the current value of the specified setting in an assigned TCP profile.",
            source: "https://clouddocs.f5.com/api/irules/PROFILE__tcp.html",
            examples: "when SERVER_CONNECTED {\n   # Log the idle timeout on the serverside TCP profile of the VIP (default of 300 seconds)\n   log local0. \"\\[PROFILE::tcp idle_timeout\\]: [PROFILE::tcp idle_timeout]\"\n}",
            return_value: "Returns the current value of the specified setting in an assigned TCP profile.",
        }),
        ..CommandSpec::DEFAULT
    }
}
