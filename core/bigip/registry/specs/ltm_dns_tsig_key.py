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
            "ltm_dns_tsig_key",
            module="ltm",
            object_types=("dns tsig-key",),
        ),
        header_types=(("ltm", "dns tsig-key"),),
        properties=(
            BigipPropertySpec(
                name="algorithm",
                value_type="enum",
                enum_values=("hmacmd5", "hmacsha1", "hmacsha256"),
            ),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="secret", value_type="string"),
        ),
    )
