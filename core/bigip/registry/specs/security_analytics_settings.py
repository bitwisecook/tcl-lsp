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
            "security_analytics_settings",
            module="security",
            object_types=("analytics settings",),
        ),
        header_types=(("security", "analytics settings"),),
        properties=(
            BigipPropertySpec(name="acl-rules", value_type="list"),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collect-client-port",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collect-dest-ip",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collect-dest-port",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collect-server-side-stats",
                value_type="enum",
                in_sections=("acl-rules",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collected-stats-external-logging",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collected-stats-internal-logging",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="dns", value_type="list"),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("dns",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="dos-l2-l4", value_type="unknown"),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("dos-l2-l4",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="l3-l4-errors", value_type="list"),
            BigipPropertySpec(
                name="collect-client-ip",
                value_type="enum",
                in_sections=("l3-l4-errors",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collect-dest-ip",
                value_type="enum",
                in_sections=("l3-l4-errors",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="publisher",
                value_type="reference",
                references=(
                    "security_blacklist_publisher_all_blacklist_publisher",
                    "security_blacklist_publisher_blacklist_publisher_stats",
                    "security_blacklist_publisher_by_addr",
                    "security_blacklist_publisher_by_category",
                    "security_blacklist_publisher_category",
                    "security_blacklist_publisher_profile",
                    "sys_icall_publisher",
                    "sys_log_config_publisher",
                ),
            ),
            BigipPropertySpec(name="smtp-config", value_type="reference"),
            BigipPropertySpec(name="stale-rules", value_type="list"),
            BigipPropertySpec(
                name="collect",
                value_type="enum",
                in_sections=("stale-rules",),
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
