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
            BigipPropertySpec(name="start-time", value_type="string"),
            BigipPropertySpec(name="end-time", value_type="string"),
            BigipPropertySpec(
                name="download-now", value_type="enum", enum_values=("true", "false")
            ),
            BigipPropertySpec(name="status", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="use-proxy", value_type="enum", enum_values=("true", "false")),
        ),
    )
