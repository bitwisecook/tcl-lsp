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
            "net_dag_globals",
            module="net",
            object_types=("dag-globals",),
        ),
        header_types=(("net", "dag-globals"),),
        properties=(
            BigipPropertySpec(
                name="round-robin-mode", value_type="enum", enum_values=("global", "local")
            ),
            BigipPropertySpec(name="dag-ipv6-prefix-len", value_type="integer"),
            BigipPropertySpec(name="icmp-hash", value_type="enum", enum_values=("icmp", "ipicmp")),
            BigipPropertySpec(
                name="icmp-monitor-priority", value_type="enum", enum_values=("high", "normal")
            ),
        ),
    )
