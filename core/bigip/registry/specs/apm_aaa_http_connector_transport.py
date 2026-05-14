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
            "apm_aaa_http_connector_transport",
            module="apm",
            object_types=("aaa http-connector-transport",),
        ),
        header_types=(("apm", "aaa http-connector-transport"),),
        properties=(),
    )
