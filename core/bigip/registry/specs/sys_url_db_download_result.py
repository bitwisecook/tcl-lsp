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
            "sys_url_db_download_result",
            module="sys",
            object_types=("url-db download-result",),
        ),
        header_types=(("sys", "url-db download-result"),),
        properties=(),
    )
