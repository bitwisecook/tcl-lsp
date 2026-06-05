//! `BWC::policy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "The bwc irule allows a bwc policy to be attached or detached to a specific flow.",
            synopsis: &["BWC::policy ('attach' | 'detach') POLICY_NAME (SESSION_ID)?"],
            snippet: "A bwc policy must exist for the given policy name, the irule will return an error if the policy cannot be found. The policy name should be give without a path name: e.g. \"gold_user\" not \"/Common/gold_user\". The irule will internally try to determine the correct pathname through lookup_folder_path_obj().\n\nOnce the irule has found the correct bwc policy name, it will know if the policy is static or dynamic. If the policy is dynamic a third arg session is required. The session is used as the bwc_cookie_t argument to the bwc public api bwc_dynamic_policy_instantiate().",
            source: "https://clouddocs.f5.com/api/irules/BWC__policy.html",
            examples: "when CLIENT_ACCEPTED {\n            BWC::policy attach gold_class\n        }",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
