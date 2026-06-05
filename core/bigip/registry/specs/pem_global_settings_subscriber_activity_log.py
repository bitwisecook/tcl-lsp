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
            "pem_global_settings_subscriber_activity_log",
            module="pem",
            object_types=("global-settings subscriber-activity-log",),
        ),
        header_types=(("pem", "global-settings subscriber-activity-log"),),
        properties=(
            BigipPropertySpec(
                name="dynamic-subscriber-ids",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
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
            BigipPropertySpec(
                name="static-subscriber-ids",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="subscriber-ip-addresses",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
        ),
    )
