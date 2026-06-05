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
            "auth_radius",
            module="auth",
            object_types=("radius",),
        ),
        header_types=(("auth", "radius"),),
        properties=(
            BigipPropertySpec(
                name="accounting-bug",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="client-id", value_type="string"),
            BigipPropertySpec(
                name="debug",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="retries", value_type="integer", default="3"),
            BigipPropertySpec(
                name="servers",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="service-type",
                value_type="enum",
                enum_values=(
                    "administrative",
                    "authenticate-only",
                    "call-check",
                    "callback-administrative",
                    "callback-framed",
                    "callback-login",
                    "callback-nas-promit",
                    "default",
                    "framed",
                    "login",
                    "nas-prompt",
                    "outbound",
                ),
                default="default, which behaves as authenticate-only",
            ),
        ),
    )
