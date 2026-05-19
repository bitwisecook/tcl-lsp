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
            "sys_ucs",
            module="sys",
            object_types=("ucs",),
        ),
        header_types=(("sys", "ucs"),),
        properties=(
            BigipPropertySpec(name="include-chassis-level-config", value_type="unknown"),
            BigipPropertySpec(
                name="load",
                value_type="reference",
                references=("gtm_global_settings_load_balancing", "load"),
            ),
            BigipPropertySpec(name="no-license", value_type="unknown"),
            BigipPropertySpec(name="no-platform-check", value_type="unknown"),
            BigipPropertySpec(name="no-private-key", value_type="unknown"),
            BigipPropertySpec(name="passphrase", value_type="unknown"),
            BigipPropertySpec(name="platform-migrate", value_type="unknown"),
            BigipPropertySpec(name="reset-trust", value_type="unknown"),
            BigipPropertySpec(name="save", value_type="reference", references=("save",)),
        ),
    )
