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
            "apm_ephemeral_auth_ssh_security_config",
            module="apm",
            object_types=("ephemeral-auth ssh-security-config",),
        ),
        header_types=(("apm", "ephemeral-auth ssh-security-config"),),
        properties=(),
    )
