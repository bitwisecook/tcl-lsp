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
            "gtm_monitor_sip",
            module="gtm",
            object_types=("monitor sip",),
        ),
        header_types=(("gtm", "monitor sip"),),
        properties=(
            BigipPropertySpec(name="cert", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="cipherlist", value_type="string"),
            BigipPropertySpec(
                name="compatibility", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("gtm_monitor_sip",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="filter",
                value_type="enum",
                allow_none=True,
                enum_values=("any", "none", "status"),
            ),
            BigipPropertySpec(
                name="filter-neg",
                value_type="enum",
                allow_none=True,
                enum_values=("any", "none", "status"),
            ),
            BigipPropertySpec(name="headers", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="ignore-down-response", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="key", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="mode", value_type="enum", enum_values=("sips", "tcp", "tls", "udp")
            ),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="request", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="username", value_type="reference", allow_none=True, references=("auth_user",)
            ),
        ),
    )
