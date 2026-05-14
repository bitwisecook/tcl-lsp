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
            "ltm_profile_mblb",
            module="ltm",
            object_types=("profile mblb",),
        ),
        header_types=(("ltm", "profile mblb"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_mblb",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="egress-high", value_type="unknown"),
            BigipPropertySpec(name="egress-low", value_type="unknown"),
            BigipPropertySpec(name="ingress-high", value_type="unknown"),
            BigipPropertySpec(name="ingress-low", value_type="unknown"),
            BigipPropertySpec(
                name="isolate-abort",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="isolate-client",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="isolate-expire",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="isolate-server",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="min-conn", value_type="unknown"),
            BigipPropertySpec(name="shutdown-timeout", value_type="unknown"),
            BigipPropertySpec(name="tag-ttl", value_type="unknown"),
        ),
    )
