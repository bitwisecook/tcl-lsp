//! `ACCESS::session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Access or manipulate session information.",
            synopsis: &["ACCESS::session create (('-flow')? ('-timeout' TIMEOUT)? ('-lifetime' LIFETIME)?)#", "ACCESS::session modify ('-sid' SESSION_ID)? (('-timeout' TIMEOUT)? (('-lifetime' LIFETIME)? | ('-remaining' REMAINING)?))#", "ACCESS::session exists ('-state_allow' | '-state_deny' | '-state_redirect' | '-state_inprogress')? (-sid)? (SESSION_ID)?", "ACCESS::session data get ('-sid' SESSION_ID)? ('-secure' | '-config')? KEY (-ssid SESSION_ID)?"],
            snippet: "The different permutations of the ACCESS::session command allow you to\naccess or manipulate different portions of session information when\ndealing with APM requests.\n\nACCESS::session data get\n\n     * Returns the value of session variable.\n\nACCESS::session data set [ ]\n\n     * Sets the value of session variable to be the given.\n\nACCESS::session exists\n\n     * This commands returns TRUE when the session with provided sid\n       exists, and returns FALSE otherwise. This command is allowed to be\n       executed in different events other then ACCESS events. This command\n       added in version 10.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__session.html",
            examples: "when ACCESS_ACL_ALLOWED {\nset user [ACCESS::session data get \"session.logon.last.username\"]\nHTTP::header insert \"X-USERNAME\" $user\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
