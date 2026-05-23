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
            "mgmt_shared_settings_api_status_log_resource_property",
            module="mgmt",
            object_types=("shared settings api-status log resource-property",),
        ),
        header_types=(("mgmt", "shared settings api-status log resource-property"),),
        properties=(),
    )
