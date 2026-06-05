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
            "apm_policy_agent_aaa_active_directory",
            module="apm",
            object_types=("policy agent aaa-active-directory",),
        ),
        header_types=(("apm", "policy agent aaa-active-directory"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="auth-max-logon-attempt", value_type="integer", default="3"),
            BigipPropertySpec(
                name="fetch-nested-groups",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="fetch-primary-groups",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="hints",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(name="options", value_type="unknown"),
            BigipPropertySpec(name="query-attrname", value_type="string", allow_none=True),
            BigipPropertySpec(name="query-filter", value_type="string", allow_none=True),
            BigipPropertySpec(name="server", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="show-extended-error",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
            BigipPropertySpec(
                name="trusted-domains",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("auth", "last", "query"),
                default="last",
            ),
            BigipPropertySpec(
                name="upn",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
                default="false",
            ),
        ),
    )
