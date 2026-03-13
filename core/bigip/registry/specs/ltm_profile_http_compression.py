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
            "ltm_profile_http_compression",
            module="ltm",
            object_types=("profile http-compression",),
        ),
        header_types=(("ltm", "profile http-compression"),),
        properties=(
            BigipPropertySpec(
                name="allow-http-10", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="browser-workarounds", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="buffer-size", value_type="integer"),
            BigipPropertySpec(
                name="cpu-saver", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="cpu-saver-high", value_type="integer"),
            BigipPropertySpec(name="cpu-saver-low", value_type="integer"),
            BigipPropertySpec(name="content-type-exclude", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="content-type-include", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_http_compression",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="gzip-level", value_type="integer"),
            BigipPropertySpec(name="gzip-memory-level", value_type="integer"),
            BigipPropertySpec(name="gzip-window-size", value_type="integer"),
            BigipPropertySpec(
                name="keep-accept-encoding", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="method-prefer", value_type="enum", enum_values=("deflate", "gzip")
            ),
            BigipPropertySpec(name="min-size", value_type="integer"),
            BigipPropertySpec(
                name="selective", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="uri-exclude", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="uri-include", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="vary-header", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
