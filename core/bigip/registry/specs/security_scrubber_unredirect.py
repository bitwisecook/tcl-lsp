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
            "security_scrubber_unredirect",
            module="security",
            object_types=("scrubber unredirect",),
        ),
        header_types=(("security", "scrubber unredirect"),),
        properties=(
            BigipPropertySpec(name="profile", value_type="string"),
            BigipPropertySpec(name="unredirect-category", value_type="string"),
            BigipPropertySpec(name="unredirect-netflow-protected-server", value_type="string"),
            BigipPropertySpec(name="unredirect-route-domain", value_type="string"),
            BigipPropertySpec(name="unredirect-virtual-server", value_type="string"),
        ),
    )
