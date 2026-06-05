//! `TCP::snd_scale` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::snd_scale",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the receive window scale advertised by the local host.",
            synopsis: &["TCP::snd_scale"],
            snippet: "Returns the receive window scale advertised by the local host.",
            source: "https://clouddocs.f5.com/api/irules/TCP__snd_scale.html",
            examples: "when CLIENT_ACCEPTED {\n    # Log snd_scale.\n    log local0. \"snd_scale: [TCP::snd_scale]\"\n}",
            return_value: "The bitshift associated with the local host window scale.",
        }),
        ..CommandSpec::DEFAULT
    }
}
