from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "sys_sflow_data_source_system",
            module="sys",
            object_types=("sflow data-source system",),
        ),
        header_types=(("sys", "sflow data-source system"),),
    )
