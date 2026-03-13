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
            "gtm_monitor_https",
            module="gtm",
            object_types=("monitor https",),
        ),
        header_types=(("gtm", "monitor https"),),
        properties=(
            BigipPropertySpec(name="cert", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="cipherlist", value_type="string"),
            BigipPropertySpec(
                name="compatibility", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("gtm_monitor_https",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(
                name="ignore-down-response", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="key", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="password",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "password"),
            ),
            BigipPropertySpec(name="probe-timeout", value_type="integer"),
            BigipPropertySpec(name="recv", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="recv-status-code", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="reverse", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="send", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="sni-server-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(
                name="transparent", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="username", value_type="reference", allow_none=True, references=("auth_user",)
            ),
        ),
    )
