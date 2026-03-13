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
            "security_ssh_profile",
            module="security",
            object_types=("ssh profile",),
        ),
        header_types=(("security", "ssh profile"),),
        properties=(
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="rules",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="actions",
                value_type="enum",
                in_sections=("rules",),
                enum_values=("add", "delete", "modify"),
            ),
            BigipPropertySpec(
                name="shell-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="sub-system-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="sftp-up-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="sftp-down-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="scp-up-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="scp-down-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="rexec-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="local-forward-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="remote-forward-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="x11-forward-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="agent-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="other-action",
                value_type="enum",
                in_sections=("actions",),
                repeated=True,
                enum_values=("allow", "disallow", "terminate", "no", "yes"),
            ),
            BigipPropertySpec(
                name="identity-users",
                value_type="enum",
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="identity-groups",
                value_type="enum",
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="actions", value_type="enum", enum_values=("add", "delete", "modify")
            ),
            BigipPropertySpec(
                name="auth-info", value_type="enum", enum_values=("add", "delete", "modify")
            ),
            BigipPropertySpec(
                name="proxy-server-auth", value_type="string", in_sections=("auth-info",)
            ),
            BigipPropertySpec(
                name="private-key", value_type="string", in_sections=("proxy-server-auth",)
            ),
            BigipPropertySpec(
                name="public-key", value_type="string", in_sections=("proxy-server-auth",)
            ),
            BigipPropertySpec(
                name="proxy-client-auth", value_type="string", in_sections=("auth-info",)
            ),
            BigipPropertySpec(
                name="private-key", value_type="string", in_sections=("proxy-client-auth",)
            ),
            BigipPropertySpec(
                name="public-key", value_type="string", in_sections=("proxy-client-auth",)
            ),
            BigipPropertySpec(
                name="real-server-auth", value_type="string", in_sections=("auth-info",)
            ),
            BigipPropertySpec(
                name="public-key", value_type="string", in_sections=("real-server-auth",)
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
            BigipPropertySpec(
                name="lang-env-tolerance",
                value_type="enum",
                allow_none=True,
                enum_values=("any", "common", "default-value", "none"),
            ),
        ),
    )
