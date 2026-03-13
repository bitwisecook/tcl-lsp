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
            "security_datasync_local_profile",
            module="security",
            object_types=("datasync local-profile",),
        ),
        header_types=(("security", "datasync local-profile"),),
        properties=(
            BigipPropertySpec(name="buf-size", value_type="integer"),
            BigipPropertySpec(
                name="ds-area",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "asm", "fps"),
            ),
            BigipPropertySpec(name="rows-bulk", value_type="integer"),
            BigipPropertySpec(name="gen-timeout-sec", value_type="integer"),
            BigipPropertySpec(name="min-mem-mb", value_type="integer"),
            BigipPropertySpec(name="min-cpu-percent", value_type="integer"),
            BigipPropertySpec(name="max-gen-rows", value_type="integer"),
            BigipPropertySpec(name="keep-conf-files", value_type="integer"),
            BigipPropertySpec(name="gen-pause-sec", value_type="integer"),
            BigipPropertySpec(
                name="offline-until-gen", value_type="enum", enum_values=("enable", "disable")
            ),
            BigipPropertySpec(name="max-iowait-percent", value_type="integer"),
        ),
    )
