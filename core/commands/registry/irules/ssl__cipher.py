# Enriched from F5 iRules reference documentation.
"""SSL::cipher -- Returns SSL cipher information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__cipher.html"


@register
class SslCipherCommand(CommandDef):
    name = "SSL::cipher"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::cipher",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns SSL cipher information.",
                synopsis=("SSL::cipher (bits | name | version |",),
                snippet="Returns an SSL cipher name, its version, and the number of secret bits used.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    # Check encryption strength\n"
                    "    if { [SSL::cipher bits] >= 128 } {\n"
                    "        pool web_servers\n"
                    "    } else {\n"
                    "        # Client is using a weak cipher\n"
                    "        # Use one of the destination commands\n"
                    "\n"
                    "        # Either specify a pool\n"
                    "        pool sorry_servers\n"
                    "\n"
                    "        # or to a specific node\n"
                    "        node 10.10.10.10\n"
                    "\n"
                    "        # or send a 302 response to redirect to a specific URL\n"
                    "        # Set cache control headers to prevent proxies from caching the response."
                ),
                return_value='SSL::cipher name Returns the current SSL cipher name using the format of the L<OpenSSL SSL_CIPHER_get_name() function|https://www.openssl.org/docs/ssl/SSL_CIPHER_get_name.html> (e.g. "EDH-RSA-DES-CBC3-SHA" or "RC4-MD5").',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::cipher (bits | name | version |",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )
