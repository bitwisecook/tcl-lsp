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
            "security_debug_register",
            module="security",
            object_types=("debug register",),
        ),
        header_types=(("security", "debug register"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="destination", value_type="unknown"),
            BigipPropertySpec(name="address", value_type="unknown", in_sections=("destination",)),
            BigipPropertySpec(name="port", value_type="unknown", in_sections=("destination",)),
            BigipPropertySpec(
                name="match-ip-version",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="protocol", value_type="unknown"),
            BigipPropertySpec(name="source", value_type="unknown"),
            BigipPropertySpec(name="address", value_type="unknown", in_sections=("source",)),
            BigipPropertySpec(name="port", value_type="unknown", in_sections=("source",)),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                in_sections=("source",),
                references=(
                    "net_fdb_vlan",
                    "net_vlan",
                    "net_vlan_allowed",
                    "net_vlan_group",
                    "sys_sflow_data_source_vlan",
                    "sys_sflow_global_settings_vlan",
                ),
            ),
        ),
    )
