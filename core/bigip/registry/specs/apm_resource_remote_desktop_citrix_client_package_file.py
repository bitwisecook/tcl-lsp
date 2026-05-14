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
            "apm_resource_remote_desktop_citrix_client_package_file",
            module="apm",
            object_types=("resource remote-desktop citrix-client-package-file",),
        ),
        header_types=(("apm", "resource remote-desktop citrix-client-package-file"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="original-file-name",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="source-path",
                value_type="string",
                required=True,
                allow_none=True,
            ),
        ),
    )
