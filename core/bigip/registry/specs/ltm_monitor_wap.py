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
            "ltm_monitor_wap",
            module="ltm",
            object_types=("monitor wap",),
        ),
        header_types=(("ltm", "monitor wap"),),
        properties=(
            BigipPropertySpec(name="accounting-node", value_type="reference"),
            BigipPropertySpec(name="accounting-port", value_type="integer", allow_none=True),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="call-id", value_type="unknown"),
            BigipPropertySpec(name="debug", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                references=("ltm_monitor_wap",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="framed-address", value_type="unknown"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="manual-resume",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="recv", value_type="string"),
            BigipPropertySpec(name="secret", value_type="unknown"),
            BigipPropertySpec(name="send", value_type="string"),
            BigipPropertySpec(name="server-id", value_type="unknown"),
            BigipPropertySpec(name="session-id", value_type="unknown"),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="up-interval", value_type="integer"),
        ),
    )
