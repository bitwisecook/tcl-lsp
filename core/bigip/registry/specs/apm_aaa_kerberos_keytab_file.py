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
            "apm_aaa_kerberos_keytab_file",
            module="apm",
            object_types=("aaa kerberos-keytab-file",),
        ),
        header_types=(("apm", "aaa kerberos-keytab-file"),),
        properties=(
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                allow_none=True,
                default="none",
            ),
            BigipPropertySpec(name="source-path", value_type="string"),
        ),
    )
