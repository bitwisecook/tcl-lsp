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
            "ltm_monitor_external",
            module="ltm",
            object_types=("monitor external",),
        ),
        header_types=(("ltm", "monitor external"),),
        properties=(
            BigipPropertySpec(name="args", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_monitor_external",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(
                name="manual-resume", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(name="user-defined", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="up-interval", value_type="integer"),
            BigipPropertySpec(name="stop", value_type="string"),
        ),
    )
