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
            "gtm_monitor_ldap",
            module="gtm",
            object_types=("monitor ldap",),
        ),
        header_types=(("gtm", "monitor ldap"),),
        properties=(
            BigipPropertySpec(name="base", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="chase-referrals", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("gtm_monitor_ldap",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="filter", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="ignore-down-response", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="mandatory-attributes", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="password",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "password"),
            ),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(
                name="security",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "ssl", "tls"),
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(
                name="username", value_type="reference", allow_none=True, references=("auth_user",)
            ),
        ),
    )
