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
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="distribution-hash",
                value_type="enum",
                enum_values=("dst-mac", "src-dst-ipport", "src-dst-mac"),
            ),
            BigipPropertySpec(name="lacp", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="lacp-mode", value_type="enum", enum_values=("active", "passive")
            ),
            BigipPropertySpec(
                name="lacp-timeout", value_type="enum", enum_values=("short", "long")
            ),
            BigipPropertySpec(
                name="link-select-policy",
                value_type="enum",
                enum_values=("auto", "maximum-bandwidth"),
            ),
            BigipPropertySpec(
                name="mac-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="stp", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="qinq-ethertype", value_type="string"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
