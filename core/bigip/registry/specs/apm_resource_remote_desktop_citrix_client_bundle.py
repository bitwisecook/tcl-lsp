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
            "apm_resource_remote_desktop_citrix_client_bundle",
            module="apm",
            object_types=("resource remote-desktop citrix-client-bundle",),
        ),
        header_types=(("apm", "resource remote-desktop citrix-client-bundle"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="download-url", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="packages", value_type="string", allow_none=True),
            BigipPropertySpec(name="sb-windows-package", value_type="string", allow_none=True),
            BigipPropertySpec(name="windows-download-url", value_type="unknown", allow_none=True),
            BigipPropertySpec(name="windows-min-version", value_type="string", allow_none=True),
            BigipPropertySpec(name="windows-package", value_type="string", allow_none=True),
        ),
    )
