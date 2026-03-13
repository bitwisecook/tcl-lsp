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
            "gtm_monitor_firepass",
            module="gtm",
            object_types=("monitor firepass",),
        ),
        header_types=(("gtm", "monitor firepass"),),
        properties=(
            BigipPropertySpec(name="cipherlist", value_type="string"),
            BigipPropertySpec(name="concurrency-limit", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("gtm_monitor_firepass",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="max-load-average", value_type="string"),
            BigipPropertySpec(
                name="password",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "password"),
            ),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="username", value_type="reference", references=("auth_user",)),
        ),
    )
