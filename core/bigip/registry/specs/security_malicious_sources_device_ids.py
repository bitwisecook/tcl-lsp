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
            "security_malicious_sources_device_ids",
            module="security",
            object_types=("malicious-sources device-ids",),
        ),
        header_types=(("security", "malicious-sources device-ids"),),
    )
