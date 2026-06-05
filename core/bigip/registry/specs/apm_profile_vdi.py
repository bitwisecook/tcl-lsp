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
            "apm_profile_vdi",
            module="apm",
            object_types=("profile vdi",),
        ),
        header_types=(("apm", "profile vdi"),),
        properties=(
            BigipPropertySpec(
                name="citrix-storefront-replacement",
                value_type="enum",
                enum_values=("disabled", "enabled"),
                shape_kind="boolean",
            ),
            BigipPropertySpec(name="msrdp-ntlm-auth-name", value_type="string", allow_none=True),
        ),
    )
