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
            "sys_sflow_data_source_vlan",
            module="sys",
            object_types=("sflow data-source vlan",),
        ),
        header_types=(("sys", "sflow data-source vlan"),),
    )
