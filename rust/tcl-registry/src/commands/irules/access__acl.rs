//! `ACCESS::acl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::acl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Poll or enforce ACLs in your connections.",
            synopsis: &["ACCESS::acl (result | matched | lookup | (eval ACL_NAME))"],
            snippet: "The ACCESS::acl commands allow you to poll, query or enforce ACLs for a\ngiven connection.\n\nACCESS::acl result\n\n     * Returns the result of ACL match for a particular URI in\n       ACCESS_ACL_ALLOWED and ACCESS_ACL_DENIED events.\n     * This result can have one of the following values\n     * - Allow\n     * - Reject\n\nACCESS::acl lookup\n\n     * Returns the name of all the assigned ACLs for a particular session.\n\nACCESS::acl eval $acl_name\n\n     * Allows admin to enforce an ACL to a user request from iRule.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__acl.html",
            examples: "when ACCESS_ACL_ALLOWED {\n      ACCESS::acl eval \"additional_acl\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ACCESS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ACCESS::acl <subcommand> ?args?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
