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
            "pem_subscriber",
            module="pem",
            object_types=("subscriber",),
        ),
        header_types=(("pem", "subscriber"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="ip-address-list",
                value_type="list",
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="policies",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="subscriber-id-type",
                value_type="enum",
                enum_values=("dhcp", "dhcp-custom", "e164", "imsi", "mac-dhcp", "nai", "private"),
            ),
        ),
    )
