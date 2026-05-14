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
            "security_scrubber_unredirect",
            module="security",
            object_types=("scrubber unredirect",),
        ),
        header_types=(("security", "scrubber unredirect"),),
        properties=(),
    )
