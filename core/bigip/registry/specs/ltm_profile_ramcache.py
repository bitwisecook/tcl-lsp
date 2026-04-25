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
            "ltm_profile_ramcache",
            module="ltm",
            object_types=("profile ramcache",),
        ),
        header_types=(("ltm", "profile ramcache"),),
        properties=(
            BigipPropertySpec(
                name="host",
                value_type="string",
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(name="max-response", value_type="integer"),
            BigipPropertySpec(name="uri", value_type="string"),
        ),
    )
