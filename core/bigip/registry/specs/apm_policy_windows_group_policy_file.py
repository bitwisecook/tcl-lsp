from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "apm_policy_windows_group_policy_file",
            module="apm",
            object_types=("policy windows-group-policy-file",),
        ),
        header_types=(("apm", "policy windows-group-policy-file"),),
        properties=(),
    )
