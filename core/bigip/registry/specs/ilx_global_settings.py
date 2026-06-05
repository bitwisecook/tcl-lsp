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
            "ilx_global_settings",
            module="ilx",
            object_types=("global-settings",),
        ),
        header_types=(("ilx", "global-settings"),),
        properties=(
            BigipPropertySpec(
                name="debug-port-blacklist",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="log-publisher", value_type="reference"),
        ),
    )
