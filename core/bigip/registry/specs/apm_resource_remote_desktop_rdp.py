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
            "apm_resource_remote_desktop_rdp",
            module="apm",
            object_types=("resource remote-desktop rdp",),
        ),
        header_types=(("apm", "resource remote-desktop rdp"),),
        properties=(
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="auto-logon",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="customization-group",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="app-service",
                value_type="string",
                in_sections=("customization-group",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="caption",
                value_type="string",
                in_sections=("customization-group",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="detailed-description",
                value_type="string",
                in_sections=("customization-group",),
                allow_none=True,
            ),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="domain-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "session.logon.last.domain"),
            ),
            BigipPropertySpec(name="host", value_type="unknown"),
            BigipPropertySpec(name="ip", value_type="string"),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="password-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "session.logon.last.password"),
            ),
            BigipPropertySpec(name="port", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="username-source",
                value_type="enum",
                allow_none=True,
                enum_values=("none",),
            ),
        ),
    )
