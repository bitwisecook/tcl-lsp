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
            "cm_sha1_fingerprint",
            module="cm",
            object_types=("sha1-fingerprint",),
        ),
        header_types=(("cm", "sha1-fingerprint"),),
    )
