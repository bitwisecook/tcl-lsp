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
            "gtm_monitor_radius",
            module="gtm",
            object_types=("monitor radius",),
        ),
        header_types=(("gtm", "monitor radius"),),
        properties=(
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("gtm_monitor_radius",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="nas-ip-address",
                value_type="boolean",
                allow_none=True,
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="password",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "password"),
            ),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(
                name="secret", value_type="enum", allow_none=True, enum_values=("none", "secret")
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(
                name="username", value_type="reference", allow_none=True, references=("auth_user",)
            ),
        ),
    )
