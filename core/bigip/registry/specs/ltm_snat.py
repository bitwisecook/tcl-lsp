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
                name="vlans",
                value_type="reference",
                allow_none=True,
                enum_values=("default", "none"),
                references=("net_vlan",),
            ),
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
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="auto-lasthop",
                value_type="enum",
                enum_values=("default", "disabled", "enabled"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="metadata", value_type="unknown"),
            BigipPropertySpec(
                name="mirror",
                value_type="unknown",
                allow_none=True,
                enum_values=("disabled", "enabled", "none"),
                shape_kind="object",
                default="none",
            ),
            BigipPropertySpec(name="origins", value_type="unknown", required=True),
            BigipPropertySpec(
                name="persist",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="snatpool",
                value_type="reference",
                references=("ltm_snatpool",),
            ),
            BigipPropertySpec(
                name="source-port",
                value_type="enum",
                enum_values=("change", "preserve", "preserve-strict"),
                default="preserve",
            ),
            BigipPropertySpec(
                name="translation",
                value_type="reference",
                repeated=True,
                references=(
                    "ltm_snat_translation",
                    "security_nat_destination_translation",
                    "security_nat_source_translation",
                ),
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(
                name="vlans",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "none"),
                default="none",
            ),
        ),
    )
