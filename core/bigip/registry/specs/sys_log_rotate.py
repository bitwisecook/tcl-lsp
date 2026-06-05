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
            "sys_log_rotate",
            module="sys",
            object_types=("log-rotate",),
        ),
        header_types=(("sys", "log-rotate"),),
        properties=(
            BigipPropertySpec(name="common-backlogs", value_type="integer", default="24"),
            BigipPropertySpec(name="common-include", value_type="string", default="none"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="ilx-include", value_type="string", default="none"),
            BigipPropertySpec(name="ilx-rotations", value_type="string", default="10"),
            BigipPropertySpec(name="ilx-schedule", value_type="string", default="daily"),
            BigipPropertySpec(name="ilx-size", value_type="string", default="10240 kilobytes"),
            BigipPropertySpec(name="include", value_type="string", default="none"),
            BigipPropertySpec(name="max-file-size", value_type="integer", default="1024000"),
            BigipPropertySpec(name="mysql-include", value_type="string"),
            BigipPropertySpec(name="syslog-include", value_type="string", default="none"),
            BigipPropertySpec(name="tomcat-include", value_type="string", default="none"),
            BigipPropertySpec(name="wa-include", value_type="string", default="none"),
        ),
    )
