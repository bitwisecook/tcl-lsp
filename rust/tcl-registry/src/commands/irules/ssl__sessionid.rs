//! `SSL::sessionid` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sessionid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets the SSL session ID.",
            synopsis: &["SSL::sessionid (desired)?"],
            snippet: "Gets the SSL session ID.",
            source: "https://clouddocs.f5.com/api/irules/SSL__sessionid.html",
            examples: "when CLIENTSSL_CLIENTCERT {\n    set cert [SSL::cert 0]\n    set sid [SSL::sessionid]\n    if { $sid ne \"\" } {\n        # If this SSL session will be cached, then it may be\n        # resumed later on a new connection. Cache the cert\n        # in the session table in case that happens. Because ID's\n        # are not globally unique, the session id needs to be combined\n        # with something from client address to avoid mismatch.\n        set key [concat [IP::remote_addr]@$sid]",
            return_value: "SSL::sessionid Returns the current connection's SSL session ID if it exists in the session cache. In version 10.x and higher, if the session ID does not exist in the cache, returns a null string. In version 9.x, if the session ID does not exist in the cache, returns a string of 64 zeroes.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::sessionid (desired)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
