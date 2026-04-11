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
            "ltm_snat",
            module="ltm",
            object_types=("snat",),
        ),
        header_types=(("ltm", "snat"),),
        properties=(
            BigipPropertySpec(
                name="auto-lasthop",
                value_type="enum",
                enum_values=("default", "enabled", "disabled"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="mirror",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("disabled", "enabled", "none"),
            ),
            BigipPropertySpec(
                name="snatpool", value_type="reference", references=("ltm_snatpool",)
            ),
            BigipPropertySpec(
                name="source-port",
                value_type="enum",
                enum_values=("change", "preserve", "preserve-strict"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="translation", value_type="list", repeated=True),
            BigipPropertySpec(
                name="vlans",
                value_type="reference",
                allow_none=True,
                enum_values=("default", "none"),
                references=("net_vlan",),
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
        ),
    )
