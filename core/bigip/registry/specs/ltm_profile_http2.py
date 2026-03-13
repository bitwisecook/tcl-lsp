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
            "ltm_profile_http2",
            module="ltm",
            object_types=("profile http2",),
        ),
        header_types=(("ltm", "profile http2"),),
        properties=(
            BigipPropertySpec(
                name="activation-modes",
                value_type="enum",
                repeated=True,
                enum_values=("alpn", "always"),
            ),
            BigipPropertySpec(name="concurrent-streams-per-connection", value_type="integer"),
            BigipPropertySpec(name="connection-idle-timeout", value_type="integer"),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_http2",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="frame-size", value_type="integer"),
            BigipPropertySpec(
                name="insert-header", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="insert-header-name", value_type="string"),
            BigipPropertySpec(name="receive-window", value_type="integer"),
            BigipPropertySpec(name="write-size", value_type="integer"),
            BigipPropertySpec(name="header-table-size", value_type="integer"),
            BigipPropertySpec(
                name="enforce-tls-requirements",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
        ),
    )
