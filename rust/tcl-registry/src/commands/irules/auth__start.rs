//! `AUTH::start` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::start",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Initializes an authentication session.",
            synopsis: &["AUTH::start TYPE SERVICE"],
            snippet: "Initializes an authentication session. This command returns the\nauthentication session ID, which must be specified to other\nauthentication commands. Multiple simultaneous authentication sessions\n(up to 10) can be opened for a single connection, but it is the user’s\nresponsibility to keep track of their respective session IDs. This\ncommand returns an error if attempted for a standby system.\n\nAUTH::start <type> <PAM service>\n\n     * Returns the authentication session ID, which must be specified to\n       other authentication commands.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__start.html",
            examples: "when CLIENT_ACCEPTED {\n  set auth_id [AUTH::start pam default_radius]\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
