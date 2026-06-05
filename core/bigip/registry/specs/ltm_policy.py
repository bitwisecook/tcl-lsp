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
            "ltm_policy",
            module="ltm",
            object_types=("policy",),
        ),
        header_types=(("ltm", "policy"),),
        properties=(
            BigipPropertySpec(name="vlan", value_type="reference", references=("net_vlan",)),
            BigipPropertySpec(name="vlan-id", value_type="reference", references=("net_vlan",)),
            BigipPropertySpec(
                name="route-domain", value_type="reference", references=("net_route_domain",)
            ),
            BigipPropertySpec(name="username", value_type="reference", references=("auth_user",)),
            BigipPropertySpec(name="with", value_type="reference", references=("auth_user",)),
            BigipPropertySpec(name="pool", value_type="reference", references=("ltm_pool",)),
            BigipPropertySpec(
                name="fallback-pool", value_type="reference", references=("ltm_pool",)
            ),
            BigipPropertySpec(name="clone-pool", value_type="reference", references=("ltm_pool",)),
            BigipPropertySpec(
                name="snatpool", value_type="reference", references=("ltm_snatpool",)
            ),
            BigipPropertySpec(name="connection", value_type="reference", references=("net_vlan",)),
            BigipPropertySpec(name="by", value_type="reference", references=("net_vlan",)),
            BigipPropertySpec(
                name="persist",
                value_type="reference",
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
            BigipPropertySpec(name="universal", value_type="reference", references=("auth_user",)),
            BigipPropertySpec(name="policy", value_type="reference", references=("ltm_policy",)),
            BigipPropertySpec(name="copy-from", value_type="reference"),
            BigipPropertySpec(name="create-draft", value_type="unknown"),
            BigipPropertySpec(
                name="rules",
                value_type="list",
                references=("ltm_rule", "ltm_rule_profiler"),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                block=(
                    BigipPropertySpec(
                        name="ordinal",
                        value_type="list",
                        in_sections=("rules",),
                        list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                    ),
                ),
            ),
            BigipPropertySpec(
                name="ordinal",
                value_type="list",
                in_sections=("rules",),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="strategy",
                value_type="enum",
                allow_none=True,
                enum_values=("STRING", "none"),
            ),
        ),
    )
