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
            "sys_url_db_download_schedule",
            module="sys",
            object_types=("url-db download-schedule",),
        ),
        header_types=(("sys", "url-db download-schedule"),),
        properties=(
            BigipPropertySpec(
                name="download-now",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="end-time", value_type="unknown"),
            BigipPropertySpec(name="start-time", value_type="unknown"),
            BigipPropertySpec(name="status", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(name="use-proxy", value_type="enum", enum_values=("false", "true")),
        ),
    )
