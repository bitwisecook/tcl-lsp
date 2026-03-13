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
            "ltm_monitor_https",
            module="ltm",
            object_types=("monitor https",),
        ),
        header_types=(("ltm", "monitor https"),),
        properties=(
            BigipPropertySpec(
                name="adaptive", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="adaptive-divergence-type",
                value_type="enum",
                enum_values=("relative", "absolute"),
            ),
            BigipPropertySpec(name="adaptive-divergence-value", value_type="integer"),
            BigipPropertySpec(name="adaptive-limit", value_type="integer"),
            BigipPropertySpec(name="adaptive-sampling-timespan", value_type="integer"),
            BigipPropertySpec(name="cert", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="cipherlist", value_type="string"),
            BigipPropertySpec(
                name="compatibility", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_monitor_https",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="ip-dscp", value_type="integer"),
            BigipPropertySpec(name="key", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="manual-resume", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="password",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "password"),
            ),
            BigipPropertySpec(name="recv", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="recv-disable", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="reverse", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="send", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="ssl-profile", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(
                name="transparent", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="up-interval", value_type="integer"),
            BigipPropertySpec(
                name="username", value_type="reference", allow_none=True, references=("auth_user",)
            ),
            BigipPropertySpec(name="stop", value_type="string"),
        ),
    )
