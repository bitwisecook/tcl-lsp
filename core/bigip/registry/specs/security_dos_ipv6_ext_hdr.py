from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "security_dos_ipv6_ext_hdr",
            module="security",
            object_types=("dos ipv6-ext-hdr",),
        ),
        header_types=(("security", "dos ipv6-ext-hdr"),),
        properties=(),
    )
