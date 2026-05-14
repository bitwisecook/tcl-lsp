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
            "pem_global_settings_session_mgmt_attributes",
            module="pem",
            object_types=("global-settings session-mgmt-attributes",),
        ),
        header_types=(("pem", "global-settings session-mgmt-attributes"),),
        properties=(
            BigipPropertySpec(
                name="clear-on-nas-reboot",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="create-event", value_type="unknown"),
            BigipPropertySpec(name="delete-event", value_type="unknown"),
            BigipPropertySpec(
                name="flow-term-on-sess-delete",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="mode", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(name="update-event", value_type="unknown"),
        ),
    )
