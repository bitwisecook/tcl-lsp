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
                name="allow-http-10",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="browser-workarounds",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="buffer-size", value_type="integer", default="4096"),
            BigipPropertySpec(
                name="content-type-exclude",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="content-type-include",
                value_type="unknown",
                allow_none=True,
                default="{ text/ application/ (xml|x-javascript) }",
            ),
            BigipPropertySpec(
                name="cpu-saver",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
            BigipPropertySpec(name="cpu-saver-high", value_type="integer", default="90"),
            BigipPropertySpec(name="cpu-saver-low", value_type="integer", default="75"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_http_compression",),
                default="httpcompression",
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="gzip-level", value_type="integer", default="1"),
            BigipPropertySpec(name="gzip-memory-level", value_type="unknown", default="8"),
            BigipPropertySpec(name="gzip-window-size", value_type="integer", default="16k"),
            BigipPropertySpec(
                name="keep-accept-encoding",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="method-prefer",
                value_type="enum",
                enum_values=("deflate", "gzip"),
                default="gzip",
            ),
            BigipPropertySpec(name="min-size", value_type="integer", default="1024"),
            BigipPropertySpec(
                name="selective",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="disabled",
            ),
            BigipPropertySpec(
                name="uri-exclude",
                value_type="unknown",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(
                name="uri-include",
                value_type="unknown",
                allow_none=True,
                default="{",
            ),
            BigipPropertySpec(
                name="vary-header",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
                default="enabled",
            ),
        ),
    )
