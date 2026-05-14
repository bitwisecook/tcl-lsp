from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_profile_rewrite",
            module="ltm",
            object_types=("profile rewrite",),
        ),
        header_types=(("ltm", "profile rewrite"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="bypass-list",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="client-caching-type",
                value_type="enum",
                enum_values=("cache-all", "cache-css-js", "cache-img-css-js", "no-cache"),
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_rewrite",),
            ),
            BigipPropertySpec(name="java-ca-file", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="java-crl", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="java-sign-key", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="java-sign-key-passphrase",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="java-signer", value_type="unknown", allow_none=True),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="rewrite-list",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="rewrite-mode",
                value_type="enum",
                enum_values=("uri-translation",),
            ),
            BigipPropertySpec(
                name="set-cookie-rules",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="client",
                value_type="unknown",
                in_sections=("set-cookie-rules",),
            ),
            BigipPropertySpec(
                name="domain",
                value_type="string",
                in_sections=("set-cookie-rules", "client"),
            ),
            BigipPropertySpec(
                name="path",
                value_type="string",
                in_sections=("set-cookie-rules", "client"),
            ),
            BigipPropertySpec(
                name="server",
                value_type="unknown",
                in_sections=("set-cookie-rules",),
            ),
            BigipPropertySpec(
                name="domain",
                value_type="string",
                in_sections=("set-cookie-rules", "server"),
            ),
            BigipPropertySpec(
                name="path",
                value_type="string",
                in_sections=("set-cookie-rules", "server"),
            ),
            BigipPropertySpec(
                name="split-tunneling",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="uri-rules",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(name="client", value_type="unknown", in_sections=("uri-rules",)),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("uri-rules", "client"),
            ),
            BigipPropertySpec(
                name="path",
                value_type="string",
                in_sections=("uri-rules", "client"),
            ),
            BigipPropertySpec(
                name="port",
                value_type="string",
                in_sections=("uri-rules", "client"),
            ),
            BigipPropertySpec(
                name="scheme",
                value_type="string",
                in_sections=("uri-rules", "client"),
            ),
            BigipPropertySpec(name="server", value_type="unknown", in_sections=("uri-rules",)),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("uri-rules", "server"),
            ),
            BigipPropertySpec(
                name="path",
                value_type="string",
                in_sections=("uri-rules", "server"),
            ),
            BigipPropertySpec(
                name="port",
                value_type="string",
                in_sections=("uri-rules", "server"),
            ),
            BigipPropertySpec(
                name="scheme",
                value_type="string",
                in_sections=("uri-rules", "server"),
            ),
        ),
    )
