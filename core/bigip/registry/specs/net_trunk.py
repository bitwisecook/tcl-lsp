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
            "net_trunk",
            module="net",
            object_types=("trunk",),
        ),
        header_types=(("net", "trunk"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="bandwidth", value_type="unknown"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="distribution-hash",
                value_type="enum",
                enum_values=("dst-mac", "src-dst-mac"),
            ),
            BigipPropertySpec(name="interfaces", value_type="unknown"),
            BigipPropertySpec(
                name="lacp",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="lacp-mode",
                value_type="enum",
                enum_values=("active", "passive"),
            ),
            BigipPropertySpec(
                name="lacp-timeout",
                value_type="enum",
                enum_values=("long", "short"),
                default="long",
            ),
            BigipPropertySpec(
                name="link-select-policy",
                value_type="enum",
                enum_values=("auto", "maximum-bandwidth"),
            ),
            BigipPropertySpec(name="mac-address", value_type="unknown"),
            BigipPropertySpec(name="qinq-ethertype", value_type="string", default="set to 0x8100"),
            BigipPropertySpec(
                name="stp",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="stp-reset", value_type="unknown"),
            BigipPropertySpec(
                name="cfg-mbr-count",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="id",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="media",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="working-mbr-count",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
