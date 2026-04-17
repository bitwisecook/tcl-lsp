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
            "ltm_virtual_address",
            module="ltm",
            object_types=("virtual-address",),
        ),
        header_types=(("ltm", "virtual-address"),),
        properties=(
            BigipPropertySpec(
                name="address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="arp", value_type="enum", enum_values=("enabled", "disabled")),
            BigipPropertySpec(name="auto-delete", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="connection-limit", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="enabled", value_type="enum", enum_values=("yes", "no")),
            BigipPropertySpec(
                name="icmp-echo",
                value_type="enum",
                enum_values=("enabled", "disabled", "selective", "always", "any", "all"),
            ),
            BigipPropertySpec(name="mask", value_type="string"),
            BigipPropertySpec(
                name="route-advertisement",
                value_type="enum",
                enum_values=("enabled", "disabled", "selective", "always", "any", "all"),
            ),
            BigipPropertySpec(
                name="server-scope",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "any", "none"),
            ),
            BigipPropertySpec(
                name="spanning", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="traffic-group",
                value_type="reference",
                allow_none=True,
                references=("cm_traffic_group",),
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(
                name="persist",
                value_type="reference",
                enum_values=("true", "false"),
                references=(
                    "ltm_persistence_cookie",
                    "ltm_persistence_dest_addr",
                    "ltm_persistence_global_settings",
                    "ltm_persistence_hash",
                    "ltm_persistence_host",
                    "ltm_persistence_msrdp",
                    "ltm_persistence_persist_records",
                    "ltm_persistence_sip",
                    "ltm_persistence_source_addr",
                    "ltm_persistence_ssl",
                    "ltm_persistence_universal",
                ),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
