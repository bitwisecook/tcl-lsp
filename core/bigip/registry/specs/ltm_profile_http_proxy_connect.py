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
            "ltm_profile_http_proxy_connect",
            module="ltm",
            object_types=("profile http-proxy-connect",),
        ),
        header_types=(("ltm", "profile http-proxy-connect"),),
        properties=(),
    )
