//! `ACCESS::uuid` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::uuid",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enumerates the session IDs that belongs to a specified uuid key by the order of its creation and provides them in a Tcl list.",
            synopsis: &[
                "ACCESS::uuid getsid SESSION_ID",
                "ACCESS::uuid ACCESS_UUID_COMMAND (ACCESS_UUID_INFO)?",
            ],
            snippet: "Enumerates the session IDs that belongs to a specified uuid key by the\norder of its creation and provides them in a Tcl list. By default, the\nuuid created by AAC is using the following format.\n  * {profile_name}.{user_name}\n\nHowever, the admin can manually override this by specifying their own\nuuid key via assigning that value to session.user.uuid session\nvariable. This can be done via iRule using ACCESS::session data set\nsession.user.uuid or via VPE using Variable Assignment Agent. The\nreturn value of ACCESS::uuid getsid is a Tcl list.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__uuid.html",
            examples: "when HTTP_REQUEST {\n    set apm_cookie_list [ ACCESS::uuid getsid \"[PROFILE::access name].[HTTP::username]\" ]\n    log local0. \"[PROFILE::access name].[HTTP::username] => session number [llength $apm_cookie_list]\"\n    for {set i 0} {$i < [llength $apm_cookie_list]} {incr i} {\n        log local0. \"MRHSession => [ lindex $apm_cookie_list $i]\"\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ACCESS::uuid getsid SESSION_ID",
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
