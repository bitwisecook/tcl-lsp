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
            "security_http_profile",
            module="security",
            object_types=("http profile",),
        ),
        header_types=(("security", "http profile"),),
        properties=(
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="evasion-techniques", value_type="string"),
            BigipPropertySpec(
                name="alarm",
                value_type="enum",
                in_sections=("evasion-techniques",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="block",
                value_type="enum",
                in_sections=("evasion-techniques",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="file-types", value_type="string"),
            BigipPropertySpec(
                name="alarm",
                value_type="enum",
                in_sections=("file-types",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="block",
                value_type="enum",
                in_sections=("file-types",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="values",
                value_type="enum",
                in_sections=("file-types",),
                repeated=True,
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="http-rfc", value_type="string"),
            BigipPropertySpec(
                name="alarm",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="bad-host-header",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="bad-version",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="block",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="body-in-get-head",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="chunked-with-content-length",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="content-length-is-positive",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="header-name-without-value",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="high-ascii-in-headers",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="host-header-is-ip",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="linefolding-in-headers",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="maximum-headers", value_type="integer", in_sections=("http-rfc",)
            ),
            BigipPropertySpec(
                name="null-in-body",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="null-in-headers",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="post-with-zero-length",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="several-content-length",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="unparsable-content",
                value_type="enum",
                in_sections=("http-rfc",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="mandatory-headers", value_type="string"),
            BigipPropertySpec(
                name="alarm",
                value_type="enum",
                in_sections=("mandatory-headers",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="block",
                value_type="enum",
                in_sections=("mandatory-headers",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="values",
                value_type="enum",
                in_sections=("mandatory-headers",),
                repeated=True,
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="maximum-length", value_type="string"),
            BigipPropertySpec(
                name="alarm",
                value_type="enum",
                in_sections=("maximum-length",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="block",
                value_type="enum",
                in_sections=("maximum-length",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="post-data", value_type="integer", in_sections=("maximum-length",)
            ),
            BigipPropertySpec(
                name="query-string", value_type="integer", in_sections=("maximum-length",)
            ),
            BigipPropertySpec(
                name="request", value_type="integer", in_sections=("maximum-length",)
            ),
            BigipPropertySpec(name="uri", value_type="integer", in_sections=("maximum-length",)),
            BigipPropertySpec(name="methods", value_type="string"),
            BigipPropertySpec(
                name="alarm",
                value_type="enum",
                in_sections=("methods",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="block",
                value_type="enum",
                in_sections=("methods",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="values",
                value_type="enum",
                in_sections=("methods",),
                repeated=True,
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="response", value_type="string"),
            BigipPropertySpec(
                name="body", value_type="boolean", in_sections=("response",), allow_none=True
            ),
            BigipPropertySpec(
                name="headers", value_type="boolean", in_sections=("response",), allow_none=True
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                in_sections=("response",),
                enum_values=("custom", "default", "redirect", "soap-fault"),
            ),
            BigipPropertySpec(
                name="url", value_type="boolean", in_sections=("response",), allow_none=True
            ),
        ),
    )
