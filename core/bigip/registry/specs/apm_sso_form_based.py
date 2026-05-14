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
            "apm_sso_form_based",
            module="apm",
            object_types=("sso form-based",),
        ),
        header_types=(("apm", "sso form-based"),),
        properties=(
            BigipPropertySpec(name="apm-log-config", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="external-access-management",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "oam"),
            ),
            BigipPropertySpec(
                name="form-action",
                value_type="unknown",
                allow_none=True,
                default="none",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="form-field",
                value_type="string",
                required=True,
                default="none",
            ),
            BigipPropertySpec(
                name="form-method",
                value_type="enum",
                enum_values=("get", "post"),
                default="post",
            ),
            BigipPropertySpec(name="form-password", value_type="string"),
            BigipPropertySpec(name="form-username", value_type="string"),
            BigipPropertySpec(
                name="headers",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                default="none",
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
                        value_type="unknown",
                        in_sections=("headers",),
                        allow_none=True,
                    ),
                    BigipPropertySpec(
                        name="hvalue",
                        value_type="integer",
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
                value_type="unknown",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="hvalue",
                value_type="integer",
                in_sections=("headers",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="password-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "session.sso.token.last.password"),
                default="session",
            ),
            BigipPropertySpec(
                name="start-uri",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="success-match-type",
                value_type="enum",
                allow_none=True,
                enum_values=("cookie", "none", "url"),
                default="none",
            ),
            BigipPropertySpec(name="success-match-value", value_type="string", default="none"),
            BigipPropertySpec(
                name="username-source",
                value_type="unknown",
                allow_none=True,
                default="session",
            ),
        ),
    )
