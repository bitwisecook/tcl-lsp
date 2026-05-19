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
            "pem_global_settings_quota_mgmt",
            module="pem",
            object_types=("global-settings quota-mgmt",),
        ),
        header_types=(("pem", "global-settings quota-mgmt"),),
        properties=(
            BigipPropertySpec(name="default-rating-group", value_type="unknown"),
            BigipPropertySpec(name="service-context-id", value_type="string"),
        ),
    )
