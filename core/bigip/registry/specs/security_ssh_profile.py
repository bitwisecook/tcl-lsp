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
            BigipPropertySpec(
                name="actions",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify")),
            ),
            BigipPropertySpec(
                name="agent-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="local-forward-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="other-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="remote-forward-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="rexec-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="scp-down-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="scp-up-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="sftp-down-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="sftp-up-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="shell-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="sub-system-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="x11-forward-action",
                value_type="unknown",
                in_sections=("actions",),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="auth-info",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify")),
            ),
            BigipPropertySpec(
                name="proxy-client-auth",
                value_type="unknown",
                in_sections=("auth-info",),
            ),
            BigipPropertySpec(
                name="private-key",
                value_type="string",
                in_sections=("auth-info", "proxy-client-auth"),
            ),
            BigipPropertySpec(
                name="public-key",
                value_type="string",
                in_sections=("auth-info", "proxy-client-auth"),
            ),
            BigipPropertySpec(
                name="proxy-server-auth",
                value_type="unknown",
                in_sections=("auth-info",),
            ),
            BigipPropertySpec(
                name="private-key",
                value_type="string",
                in_sections=("auth-info", "proxy-server-auth"),
            ),
            BigipPropertySpec(
                name="public-key",
                value_type="string",
                in_sections=("auth-info", "proxy-server-auth"),
            ),
            BigipPropertySpec(
                name="real-server-auth",
                value_type="unknown",
                in_sections=("auth-info",),
            ),
            BigipPropertySpec(
                name="public-key",
                value_type="string",
                in_sections=("auth-info", "real-server-auth"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="lang-env-tolerance",
                value_type="enum",
                allow_none=True,
                enum_values=("any", "common", "default-value", "none"),
            ),
            BigipPropertySpec(
                name="rules",
                value_type="list",
                allow_none=True,
                references=(
                    "gtm_rule",
                    "ltm_cipher_rule",
                    "ltm_global_settings_rule",
                    "ltm_rule",
                    "ltm_rule_profiler",
                    "security_firewall_matching_rule",
                    "security_firewall_on_demand_rule_deploy",
                    "security_firewall_rule_list",
                    "security_firewall_rule_stat",
                    "security_packet_filter_rule_stat",
                    "sys_file_rewrite_rule",
                ),
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="actions",
                value_type="list",
                in_sections=("rules",),
                list_operators=frozenset(("add", "delete", "modify")),
            ),
            BigipPropertySpec(
                name="agent-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="local-forward-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="other-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="remote-forward-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="rexec-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="scp-down-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="scp-up-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="sftp-down-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="sftp-up-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="shell-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="sub-system-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(
                name="x11-forward-action",
                value_type="unknown",
                in_sections=("rules", "actions"),
                enum_values=("allow", "disallow", "no", "terminate", "yes"),
            ),
            BigipPropertySpec(name="description", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="identity-groups",
                value_type="list",
                in_sections=("rules",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="identity-users",
                value_type="list",
                in_sections=("rules",),
                list_operators=frozenset(("add", "delete", "replace-all-with")),
            ),
            BigipPropertySpec(name="timeout", value_type="integer"),
        ),
    )
