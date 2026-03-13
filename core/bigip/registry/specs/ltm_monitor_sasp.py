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
            "ltm_monitor_sasp",
            module="ltm",
            object_types=("monitor sasp",),
        ),
        header_types=(("ltm", "monitor sasp"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from", value_type="reference", references=("ltm_monitor_sasp",)
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="interval", value_type="integer"),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("pull", "push")),
            BigipPropertySpec(
                name="primary-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="protocol", value_type="enum", enum_values=("tcp", "udp")),
            BigipPropertySpec(
                name="secondary-address",
                value_type="boolean",
                allow_none=True,
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="service", value_type="enum", allow_none=True, enum_values=("none", "port")
            ),
            BigipPropertySpec(name="time-until-up", value_type="integer"),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
