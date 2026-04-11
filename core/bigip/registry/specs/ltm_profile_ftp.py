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
            "ltm_profile_ftp",
            module="ltm",
            object_types=("profile ftp",),
        ),
        header_types=(("ltm", "profile ftp"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_ftp",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="port", value_type="integer", min_value=0, max_value=65535),
            BigipPropertySpec(
                name="allow-ftps", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="ftps-mode", value_type="enum", enum_values=("disallow", "allow", "require")
            ),
            BigipPropertySpec(
                name="enforce-tls-session-reuse",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="allow-active-mode", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="security", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="translate-extended", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="inherit-parent-profile",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-profile", value_type="boolean", allow_none=True),
        ),
    )
