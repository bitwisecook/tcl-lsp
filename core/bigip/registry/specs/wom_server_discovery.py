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
            "wom_server_discovery",
            module="wom",
            object_types=("server-discovery",),
        ),
        header_types=(("wom", "server-discovery"),),
        properties=(
            BigipPropertySpec(
                name="auto-save",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="filter-mode",
                value_type="enum",
                enum_values=("exclude", "include"),
            ),
            BigipPropertySpec(name="idle-time-limit", value_type="integer"),
            BigipPropertySpec(name="ip-ttl-limit", value_type="integer"),
            BigipPropertySpec(name="max-server-count", value_type="integer"),
            BigipPropertySpec(name="min-idle-time", value_type="integer"),
            BigipPropertySpec(name="min-prefix-length-ipv4", value_type="integer"),
            BigipPropertySpec(name="min-prefix-length-ipv6", value_type="integer"),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="rtt-threshold", value_type="integer"),
            BigipPropertySpec(
                name="subnet-filter",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="time-unit",
                value_type="enum",
                enum_values=("days", "hours", "minutes"),
            ),
        ),
    )
