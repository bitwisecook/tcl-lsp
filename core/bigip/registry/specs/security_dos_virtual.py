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
            "security_dos_virtual",
            module="security",
            object_types=("dos virtual",),
        ),
        header_types=(("security", "dos virtual"),),
        properties=(BigipPropertySpec(name="dns-nxdomain-stat", value_type="unknown"),),
    )
