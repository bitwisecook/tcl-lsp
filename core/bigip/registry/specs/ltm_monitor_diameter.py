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
            "ltm_monitor_diameter",
            module="ltm",
            object_types=("monitor diameter",),
        ),
        header_types=(("ltm", "monitor diameter"),),
        properties=(
            BigipPropertySpec(name="acct-application-id", value_type="integer", allow_none=True),
            BigipPropertySpec(name="auth-application-id", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_monitor_diameter",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="host-ip-address",
                value_type="boolean",
                allow_none=True,
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="manual-resume", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="mode", value_type="enum", enum_values=("tcp", "mr-tcp", "mr-sctp")
            ),
            BigipPropertySpec(name="origin-host", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="origin-realm", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="product-name", value_type="string"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="up-interval", value_type="integer"),
            BigipPropertySpec(name="vendor-id", value_type="integer"),
            BigipPropertySpec(
                name="vendor-specific-acct-application-id", value_type="integer", allow_none=True
            ),
            BigipPropertySpec(
                name="vendor-specific-auth-application-id", value_type="integer", allow_none=True
            ),
            BigipPropertySpec(
                name="vendor-specific-vendor-id", value_type="integer", allow_none=True
            ),
        ),
    )
