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
            "ltm_profile_certificate_authority",
            module="ltm",
            object_types=("profile certificate-authority",),
        ),
        header_types=(("ltm", "profile certificate-authority"),),
        properties=(
            BigipPropertySpec(name="authenticate-depth", value_type="unknown"),
            BigipPropertySpec(name="ca-file", value_type="unknown"),
            BigipPropertySpec(name="crl-file", value_type="unknown"),
            BigipPropertySpec(name="default-name", value_type="unknown"),
            BigipPropertySpec(name="description", value_type="unknown"),
            BigipPropertySpec(name="update-crl", value_type="unknown"),
        ),
    )
