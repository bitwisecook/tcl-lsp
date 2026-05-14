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
            "security_debug_matcher",
            module="security",
            object_types=("debug matcher",),
        ),
        header_types=(("security", "debug matcher"),),
        properties=(
            BigipPropertySpec(name="matcher", value_type="unknown"),
            BigipPropertySpec(name="drop-redirect", value_type="unknown", in_sections=("matcher",)),
            BigipPropertySpec(
                name="drop-redirect-mode",
                value_type="unknown",
                in_sections=("matcher", "drop-redirect"),
            ),
            BigipPropertySpec(
                name="disable",
                value_type="unknown",
                in_sections=("matcher", "drop-redirect", "drop-redirect-mode"),
            ),
            BigipPropertySpec(
                name="redirect-all",
                value_type="unknown",
                in_sections=("matcher", "drop-redirect", "drop-redirect-mode"),
            ),
            BigipPropertySpec(
                name="redirect-hw-only",
                value_type="unknown",
                in_sections=("matcher", "drop-redirect", "drop-redirect-mode"),
            ),
            BigipPropertySpec(
                name="redirect-sw-only",
                value_type="unknown",
                in_sections=("matcher", "drop-redirect", "drop-redirect-mode"),
            ),
        ),
    )
