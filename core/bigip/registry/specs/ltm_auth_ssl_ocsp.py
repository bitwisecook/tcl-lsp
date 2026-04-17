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
            "ltm_auth_ssl_ocsp",
            module="ltm",
            object_types=("auth ssl-ocsp",),
        ),
        header_types=(("ltm", "auth ssl-ocsp"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="responders",
                value_type="enum",
                allow_none=True,
                enum_values=("default", "none"),
            ),
        ),
    )
