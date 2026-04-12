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
            "net_multicast_globals",
            module="net",
            object_types=("multicast-globals",),
        ),
        header_types=(("net", "multicast-globals"),),
        properties=(
            BigipPropertySpec(name="route-lookup-timeout", value_type="integer"),
            BigipPropertySpec(name="max-pending-routes", value_type="integer"),
            BigipPropertySpec(name="max-pending-packets", value_type="integer"),
            BigipPropertySpec(
                name="rate-limit", value_type="enum", enum_values=("disabled", "enabled")
            ),
        ),
    )
