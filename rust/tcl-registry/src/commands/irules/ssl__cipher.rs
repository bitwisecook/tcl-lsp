//! `SSL::cipher` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::cipher",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns SSL cipher information.",
            synopsis: &["SSL::cipher bits", "SSL::cipher name", "SSL::cipher version", "SSL::cipher clientlist"],
            snippet: "Returns an SSL cipher name, its version, and the number of secret bits used.",
            source: "https://clouddocs.f5.com/api/irules/SSL__cipher.html",
            examples: "when HTTP_REQUEST {\n    # Check encryption strength\n    if { [SSL::cipher bits] >= 128 } {\n        pool web_servers\n    } else {\n        # Client is using a weak cipher\n        # Use one of the destination commands\n\n        # Either specify a pool\n        pool sorry_servers\n\n        # or to a specific node\n        node 10.10.10.10\n\n        # or send a 302 response to redirect to a specific URL\n        # Set cache control headers to prevent proxies from caching the response.",
            return_value: "SSL::cipher name Returns the current SSL cipher name using the format of the L<OpenSSL SSL_CIPHER_get_name() function|https://www.openssl.org/docs/ssl/SSL_CIPHER_get_name.html> (e.g. \"EDH-RSA-DES-CBC3-SHA\" or \"RC4-MD5\").",
        }),
        ..CommandSpec::DEFAULT
    }
}
