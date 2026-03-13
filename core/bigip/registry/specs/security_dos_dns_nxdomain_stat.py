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
            "security_dos_dns_nxdomain_stat",
            module="security",
            object_types=("dos dns-nxdomain-stat",),
        ),
        header_types=(("security", "dos dns-nxdomain-stat"),),
        properties=(BigipPropertySpec(name="reset-stats", value_type="string"),),
    )
