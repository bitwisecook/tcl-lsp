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
            "apm_policy_agent_aaa_localdb",
            module="apm",
            object_types=("policy agent aaa-localdb",),
        ),
        header_types=(("apm", "policy agent aaa-localdb"),),
        properties=(),
    )
