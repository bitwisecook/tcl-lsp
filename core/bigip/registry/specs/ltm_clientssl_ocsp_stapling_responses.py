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
            "ltm_clientssl_ocsp_stapling_responses",
            module="ltm",
            object_types=("clientssl ocsp-stapling-responses",),
        ),
        header_types=(("ltm", "clientssl ocsp-stapling-responses"),),
        properties=(
            BigipPropertySpec(name="virtual", value_type="string"),
            BigipPropertySpec(name="clientssl-profile", value_type="string"),
        ),
    )
