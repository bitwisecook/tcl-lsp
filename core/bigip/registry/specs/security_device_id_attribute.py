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
            "security_device_id_attribute",
            module="security",
            object_types=("device-id attribute",),
        ),
        header_types=(("security", "device-id attribute"),),
        properties=(
            BigipPropertySpec(name="collect", value_type="unknown", shape_kind="object"),
            BigipPropertySpec(
                name="id",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )
