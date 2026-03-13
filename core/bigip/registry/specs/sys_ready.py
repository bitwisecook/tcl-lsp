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
            "sys_ready",
            module="sys",
            object_types=("ready",),
        ),
        header_types=(("sys", "ready"),),
        properties=(
            BigipPropertySpec(name="config", value_type="boolean"),
            BigipPropertySpec(name="license", value_type="boolean"),
            BigipPropertySpec(name="provision", value_type="boolean"),
            BigipPropertySpec(name="sys", value_type="string"),
            BigipPropertySpec(name="config-ready", value_type="boolean", in_sections=("sys",)),
            BigipPropertySpec(name="license-ready", value_type="boolean", in_sections=("sys",)),
            BigipPropertySpec(name="provision-ready", value_type="boolean", in_sections=("sys",)),
        ),
    )
