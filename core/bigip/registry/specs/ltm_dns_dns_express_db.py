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
            "ltm_dns_dns_express_db",
            module="ltm",
            object_types=("dns dns-express-db",),
        ),
        header_types=(("ltm", "dns dns-express-db"),),
        properties=(BigipPropertySpec(name="load", value_type="string"),),
    )
