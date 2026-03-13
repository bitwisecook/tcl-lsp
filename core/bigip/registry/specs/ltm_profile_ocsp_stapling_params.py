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
            "ltm_profile_ocsp_stapling_params",
            module="ltm",
            object_types=("profile ocsp-stapling-params",),
        ),
        header_types=(("ltm", "profile ocsp-stapling-params"),),
    )
