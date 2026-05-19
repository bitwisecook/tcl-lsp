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
            "ltm_monitor_ldap",
            module="ltm",
            object_types=("monitor ldap",),
        ),
        header_types=(("ltm", "monitor ldap"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="base", value_type="string", default="none"),
            BigipPropertySpec(
                name="chase-referrals",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="yes",
            ),
            BigipPropertySpec(
                name="debug",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_ldap",),
                default="ldap",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="destination",
                value_type="string",
                shape_kind="endpoint",
                default="*:*",
            ),
            BigipPropertySpec(name="filter", value_type="unknown", allow_none=True, default="none"),
            BigipPropertySpec(name="interval", value_type="integer", default="10 seconds"),
            BigipPropertySpec(
                name="mandatory-attributes",
                value_type="enum",
                enum_values=("no", "yes"),
                shape_kind="boolean",
                default="no",
            ),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="password", value_type="unknown", default="none"),
            BigipPropertySpec(
                name="security",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "ssl", "tls"),
                default="none",
            ),
            BigipPropertySpec(name="time-until-up", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="timeout", value_type="integer", default="31 seconds"),
            BigipPropertySpec(
                name="up-interval",
                value_type="integer",
                default="0 (zero), which specifies that the system uses the value of the interval option whether the resource is up or down",
            ),
            BigipPropertySpec(
                name="username",
                value_type="reference",
                allow_none=True,
                default="none",
            ),
        ),
    )
