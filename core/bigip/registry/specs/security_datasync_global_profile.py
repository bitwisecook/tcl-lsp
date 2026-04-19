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
            "security_datasync_global_profile",
            module="security",
            object_types=("datasync global-profile",),
        ),
        header_types=(("security", "datasync global-profile"),),
        properties=(
            BigipPropertySpec(name="table", value_type="string"),
            BigipPropertySpec(name="activation-epoch", value_type="integer"),
            BigipPropertySpec(name="deactivation-epoch", value_type="integer"),
            BigipPropertySpec(name="min-rows", value_type="integer"),
            BigipPropertySpec(name="max-rows", value_type="integer"),
            BigipPropertySpec(name="regen-time-offset", value_type="integer"),
            BigipPropertySpec(name="regen-interval", value_type="integer", allow_none=True),
            BigipPropertySpec(name="grace-time", value_type="integer"),
            BigipPropertySpec(name="master-key", value_type="string"),
            BigipPropertySpec(name="scramble-alg", value_type="string"),
            BigipPropertySpec(name="hash-alg", value_type="string"),
            BigipPropertySpec(name="mac-alg", value_type="string"),
            BigipPropertySpec(name="mode-of-op", value_type="string"),
            BigipPropertySpec(
                name="rsa-exp",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "rsa-3", "rsa-f4", "default"),
            ),
            BigipPropertySpec(name="rsa-bits", value_type="integer", allow_none=True),
            BigipPropertySpec(name="params", value_type="string"),
        ),
    )
