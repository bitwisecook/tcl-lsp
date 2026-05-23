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
            "net_fdb_vlan",
            module="net",
            object_types=("fdb vlan",),
        ),
        header_types=(("net", "fdb vlan"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="interface",
                value_type="reference",
                references=(
                    "net_interface",
                    "net_interface_cos",
                    "net_interface_ddm",
                    "sys_sflow_data_source_interface",
                    "sys_sflow_global_settings_interface",
                ),
            ),
            BigipPropertySpec(name="records", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="trunk", value_type="reference", references=("net_trunk",)),
        ),
    )
