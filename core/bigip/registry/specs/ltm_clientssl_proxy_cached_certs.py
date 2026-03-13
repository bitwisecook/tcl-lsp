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
            "ltm_clientssl_proxy_cached_certs",
            module="ltm",
            object_types=("clientssl-proxy cached-certs",),
        ),
        header_types=(("ltm", "clientssl-proxy cached-certs"),),
        properties=(
            BigipPropertySpec(name="virtual", value_type="string"),
            BigipPropertySpec(name="clientssl-profile", value_type="string"),
        ),
    )
