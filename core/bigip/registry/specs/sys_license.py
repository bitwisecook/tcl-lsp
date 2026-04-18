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
            "sys_license",
            module="sys",
            object_types=("license",),
        ),
        header_types=(("sys", "license"),),
        properties=(
            BigipPropertySpec(name="install", value_type="string"),
            BigipPropertySpec(name="add-on-keys", value_type="list", repeated=True),
            BigipPropertySpec(name="license-server", value_type="string"),
            BigipPropertySpec(
                name="license-server-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="registration-key", value_type="string"),
            BigipPropertySpec(name="revoke", value_type="string"),
        ),
    )
