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
            "apm_epsec_epsec_package",
            module="apm",
            object_types=("epsec epsec-package",),
        ),
        header_types=(("apm", "epsec epsec-package"),),
        properties=(
            BigipPropertySpec(name="local-path", value_type="string", required=True),
            BigipPropertySpec(name="server", value_type="string", allow_none=True),
        ),
    )
