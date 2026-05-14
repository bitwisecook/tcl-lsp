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
            "cm_device_group",
            module="cm",
            object_types=("device-group",),
        ),
        header_types=(("cm", "device-group"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="asm-sync",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-sync",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="clear-incremental-config-sync-cache", value_type="unknown"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="devices",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="full-load-on-sync",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="incremental-config-sync-size-max", value_type="integer"),
            BigipPropertySpec(
                name="network-failover",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="save-on-auto-sync",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=("sync-failover", "sync-only"),
            ),
        ),
    )
