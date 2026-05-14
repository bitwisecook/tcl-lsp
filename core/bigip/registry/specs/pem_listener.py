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
            "pem_listener",
            module="pem",
            object_types=("listener",),
        ),
        header_types=(("pem", "listener"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="profile-spm", value_type="reference"),
            BigipPropertySpec(name="profile-subscriber-mgmt", value_type="reference"),
            BigipPropertySpec(
                name="virtual-servers",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
        ),
    )
