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
            BigipPropertySpec(
                name="bypass-list",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("add", "delete", "replace-all-with", "none"),
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
            BigipPropertySpec(name="java-ca-file", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="java-crl", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="java-sign-key", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="java-sign-key-passphrase", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(name="java-signer", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="location-specific", value_type="enum", enum_values=("false", "true")
            ),
            BigipPropertySpec(
                name="rewrite-list",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("add", "delete", "replace-all-with", "none"),
            ),
            BigipPropertySpec(
                name="rewrite-mode", value_type="enum", enum_values=("portal", "uri-translation")
            ),
            BigipPropertySpec(
                name="set-cookie-rules",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "replace-all-with", "none"),
            ),
            BigipPropertySpec(
                name="client", value_type="string", in_sections=("set-cookie-rules",)
            ),
            BigipPropertySpec(name="domain", value_type="string", in_sections=("client",)),
            BigipPropertySpec(name="path", value_type="string", in_sections=("client",)),
            BigipPropertySpec(
                name="server",
                value_type="string",
                in_sections=("set-cookie-rules",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="domain", value_type="string", in_sections=("server",)),
            BigipPropertySpec(name="path", value_type="string", in_sections=("server",)),
            BigipPropertySpec(
                name="split-tunneling", value_type="enum", enum_values=("false", "true")
            ),
            BigipPropertySpec(
                name="uri-rules",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "replace-all-with", "none"),
            ),
            BigipPropertySpec(name="client", value_type="string", in_sections=("uri-rules",)),
            BigipPropertySpec(name="scheme", value_type="string", in_sections=("client",)),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("client",),
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("client",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="server",
                value_type="string",
                in_sections=("uri-rules",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="scheme", value_type="string", in_sections=("server",)),
            BigipPropertySpec(
                name="host",
                value_type="string",
                in_sections=("server",),
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("server",),
                min_value=0,
                max_value=65535,
            ),
        ),
    )
