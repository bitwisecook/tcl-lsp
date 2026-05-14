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
            "apm_aaa_http",
            module="apm",
            object_types=("aaa http",),
        ),
        header_types=(("apm", "aaa http"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auth-type",
                value_type="enum",
                enum_values=("basic-ntlm", "custom-post", "form-based"),
            ),
            BigipPropertySpec(
                name="content-type",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "url-encoded-utf8", "xml-utf8"),
            ),
            BigipPropertySpec(name="custom-body", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="description",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="follow-redirect", value_type="integer"),
            BigipPropertySpec(
                name="form-action",
                value_type="string",
                allow_none=True,
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="form-fields",
                value_type="string",
                required=True,
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="form-method",
                value_type="enum",
                enum_values=("get", "post"),
                default="POST",
            ),
            BigipPropertySpec(name="form-params", value_type="string", allow_none=True),
            BigipPropertySpec(name="form-password", value_type="string", allow_none=True),
            BigipPropertySpec(name="form-username", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="headers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="app-service",
                        value_type="string",
                        in_sections=("headers",),
                        allow_none=True,
                        default="none",
                    ),
                    BigipPropertySpec(
                        name="hname",
                        value_type="string",
                        in_sections=("headers",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="hvalue",
                        value_type="string",
                        in_sections=("headers",),
                        allow_none=True,
                    ),
                ),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("headers",),
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="hname",
                value_type="string",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="hvalue",
                value_type="string",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="start-uri", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="success-match-type",
                value_type="enum",
                enum_values=("cookie", "exact-cookie", "url"),
            ),
            BigipPropertySpec(name="success-match-value", value_type="string", allow_none=True),
        ),
    )
