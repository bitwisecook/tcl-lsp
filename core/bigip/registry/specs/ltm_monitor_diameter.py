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
            BigipPropertySpec(
                name="acct-application-id",
                value_type="integer",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auth-application-id",
                value_type="integer",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_diameter",),
                default="firepass",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="host-ip-address",
                value_type="string",
                allow_none=True,
                shape_kind="ip-address",
                default="none",
            ),
            BigipPropertySpec(name="interval", value_type="integer", default="10 seconds"),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                enum_values=("mr-sctp", "mr-tcp", "tcp"),
            ),
            BigipPropertySpec(
                name="origin-host",
                value_type="string",
                allow_none=True,
                shape_kind="ip-address",
                default="the fully-qualified domain name of the BIG-IP system",
            ),
            BigipPropertySpec(
                name="origin-realm",
                value_type="unknown",
                allow_none=True,
                default="f5",
            ),
            BigipPropertySpec(
                name="product-name",
                value_type="reference",
                default="F5 BIGIP Diameter Health Monitoring",
            ),
            BigipPropertySpec(name="time-until-up", value_type="integer", default="0 (zero)"),
            BigipPropertySpec(name="timeout", value_type="integer", default="31 seconds"),
            BigipPropertySpec(
                name="up-interval",
                value_type="integer",
                default="0 (zero), which specifies that the system uses the value of the interval option whether the resource is up or down",
            ),
            BigipPropertySpec(name="vendor-id", value_type="integer", default="3375"),
            BigipPropertySpec(
                name="vendor-specific-acct-application-id",
                value_type="integer",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="vendor-specific-auth-application-id",
                value_type="integer",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="vendor-specific-vendor-id",
                value_type="integer",
                allow_none=True,
                default="none",
            ),
        ),
    )
