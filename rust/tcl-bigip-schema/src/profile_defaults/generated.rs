// @generated — DO NOT EDIT. Regenerate to refresh.

use super::{BigipVersion, FieldDefault, ProfileDefaults, VersionRange};

/// Profile field defaults for one snapshot version. The resolver floor-matches
/// a report's version to the nearest not-newer snapshot. See [`super::PROFILE_DEFAULTS`].
pub static PROFILE_DEFAULTS_GENERATED: &[ProfileDefaults] = &[
    ProfileDefaults {
        profile: "AIMCP",
        tmsh_kind: "ltm profile aimcp",
        fields: &[],
    },
    ProfileDefaults {
        profile: "ANALYTICS",
        tmsh_kind: "ltm profile analytics",
        fields: &[
            FieldDefault {
                field: "collect-server-latency",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-page-load-time",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-url",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-ip",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-geo",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-user-agent",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-http-throughput",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-response-codes",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-methods",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-max-tps-and-throughput",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "publish-irule-statistics",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-user-sessions",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "session-timeout",
                value: "300",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collected-stats-internal-logging",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "captured-traffic-internal-logging",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collected-stats-external-logging",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "captured-traffic-external-logging",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "notification-by-syslog",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "notification-by-snmp",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "notification-by-email",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "CERTIFICATEAUTHORITY",
        tmsh_kind: "ltm profile certificate-authority",
        fields: &[
            FieldDefault {
                field: "ca-file",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "crl-file",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "authenticate-depth",
                value: "9",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "update-crl",
                value: "false",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "CLASSIFICATION",
        tmsh_kind: "ltm profile classification",
        fields: &[
            FieldDefault {
                field: "preset",
                value: "/Common/ce",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "irule-event",
                value: "on",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "app-detection",
                value: "on",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "urlcat",
                value: "off",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "CLIENTLDAP",
        tmsh_kind: "ltm profile client-ldap",
        fields: &[FieldDefault {
            field: "activation-mode",
            value: "require",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "CLIENTSSL",
        tmsh_kind: "ltm profile client-ssl",
        fields: &[
            FieldDefault {
                field: "alert-timeout",
                value: "indefinite",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "allow-dynamic-record-sizing",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "authenticate",
                value: "once",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "authenticate-depth",
                value: "9",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "ca-file",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-size",
                value: "262144",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-timeout",
                value: "3600",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cert-extension-includes",
                value: "basic-constraints subject-alternative-name",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cert-key-chain",
                value: "{ default { cert /Common/default.crt chain none key /Common/default.key passphrase none } }",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cert",
                value: "/Common/default.crt",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "chain",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cipher-group",
                value: "none",
                range: VersionRange::between(
                    BigipVersion::new(17, 1, 0, 0),
                    BigipVersion::new(21, 1, 0, 0),
                ),
            },
            FieldDefault {
                field: "cipher-group",
                value: "/Common/f5-default",
                range: VersionRange::from(BigipVersion::new(21, 1, 0, 0)),
            },
            FieldDefault {
                field: "ciphers",
                value: "DEFAULT",
                range: VersionRange::between(
                    BigipVersion::new(17, 1, 0, 0),
                    BigipVersion::new(21, 1, 0, 0),
                ),
            },
            FieldDefault {
                field: "ciphers",
                value: "none",
                range: VersionRange::from(BigipVersion::new(21, 1, 0, 0)),
            },
            FieldDefault {
                field: "client-cert-ca",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "crl-file",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "handshake-timeout",
                value: "10",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "key",
                value: "/Common/default.key",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "maximum-record-size",
                value: "16384",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "mod-ssl-methods",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "mode",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "options",
                value: "dont-insert-empty-fragments no-tlsv1.3 no-dtlsv1.2",
                range: VersionRange::between(
                    BigipVersion::new(17, 1, 0, 0),
                    BigipVersion::new(21, 1, 0, 0),
                ),
            },
            FieldDefault {
                field: "options",
                value: "dont-insert-empty-fragments no-tlsv1.1 no-tlsv1 no-ssl",
                range: VersionRange::from(BigipVersion::new(21, 1, 0, 0)),
            },
            FieldDefault {
                field: "passphrase",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "peer-cert-mode",
                value: "ignore",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "renegotiate-max-record-delay",
                value: "indefinite",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "renegotiate-period",
                value: "indefinite",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "renegotiate-size",
                value: "indefinite",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "renegotiation",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "secure-renegotiation",
                value: "require",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "strict-resume",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "unclean-shutdown",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "peer-no-renegotiate-timeout",
                value: "10",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "log-publisher",
                value: "/Common/sys-ssl-publisher",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "CONNECTOR",
        tmsh_kind: "ltm profile connector",
        fields: &[],
    },
    ProfileDefaults {
        profile: "DHCPV4",
        tmsh_kind: "ltm profile dhcpv4",
        fields: &[
            FieldDefault {
                field: "idle-timeout",
                value: "60",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "default-lease-time",
                value: "86400",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "transaction-timeout",
                value: "30",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "max-hops",
                value: "4",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "ttl-value",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "DHCPV6",
        tmsh_kind: "ltm profile dhcpv6",
        fields: &[
            FieldDefault {
                field: "idle-timeout",
                value: "60",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "default-lease-time",
                value: "86400",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "transaction-timeout",
                value: "30",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "DIAMETER",
        tmsh_kind: "ltm profile diameter",
        fields: &[FieldDefault {
            field: "persist-avp",
            value: "Session-Id",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "DNS",
        tmsh_kind: "ltm profile dns",
        fields: &[FieldDefault {
            field: "enable-gtm",
            value: "yes",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "DNSACCELERATION",
        tmsh_kind: "ltm profile dns-acceleration",
        fields: &[],
    },
    ProfileDefaults {
        profile: "DOHPROXY",
        tmsh_kind: "ltm profile doh-proxy",
        fields: &[],
    },
    ProfileDefaults {
        profile: "DOHSERVER",
        tmsh_kind: "ltm profile doh-server",
        fields: &[],
    },
    ProfileDefaults {
        profile: "FASTHTTP",
        tmsh_kind: "ltm profile fasthttp",
        fields: &[
            FieldDefault {
                field: "client-close-timeout",
                value: "5",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "connpool-idle-timeout-override",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "connpool-max-reuse",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "connpool-max-size",
                value: "2048",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "connpool-min-size",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "connpool-step",
                value: "4",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "header-insert",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "idle-timeout",
                value: "300",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "insert-xforwarded-for",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "layer-7",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "max-header-size",
                value: "32768",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "max-requests",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "mss-override",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "reset-on-timeout",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "server-close-timeout",
                value: "5",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "FASTL4",
        tmsh_kind: "ltm profile fastl4",
        fields: &[
            FieldDefault {
                field: "idle-timeout",
                value: "300",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "mss-override",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "pva-acceleration",
                value: "full",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "reassemble-fragments",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "reset-on-timeout",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "FIX",
        tmsh_kind: "ltm profile fix",
        fields: &[],
    },
    ProfileDefaults {
        profile: "FTP",
        tmsh_kind: "ltm profile ftp",
        fields: &[],
    },
    ProfileDefaults {
        profile: "GEOREDUNDANCY",
        tmsh_kind: "ltm profile georedundancy",
        fields: &[],
    },
    ProfileDefaults {
        profile: "GTP",
        tmsh_kind: "ltm profile gtp",
        fields: &[],
    },
    ProfileDefaults {
        profile: "HTML",
        tmsh_kind: "ltm profile html",
        fields: &[FieldDefault {
            field: "content-selection",
            value: "text/html text/xhtml",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "HTTP",
        tmsh_kind: "ltm profile http",
        fields: &[
            FieldDefault {
                field: "basic-auth-realm",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "lws-width",
                value: "80",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "oneconnect-transformations",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "oneconnect-status-reuse",
                value: "\"200 206\"",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "proxy-type",
                value: "reverse",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "enforcement",
                value: "max-header-count 64 max-header-size 32768 pipeline allow unknown-method allow",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "request-chunking",
                value: "sustain",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "response-chunking",
                value: "sustain",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "HTTPCOMPRESSION",
        tmsh_kind: "ltm profile http-compression",
        fields: &[
            FieldDefault {
                field: "allow-http-10",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "buffer-size",
                value: "4096",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "content-type-exclude",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "content-type-include",
                value: "text/ \"application/(xml|x-javascript)\"",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cpu-saver",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cpu-saver-high",
                value: "90",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cpu-saver-low",
                value: "75",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "gzip-level",
                value: "1",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "gzip-memory-level",
                value: "8k",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "gzip-window-size",
                value: "16k",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "keep-accept-encoding",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "method-prefer",
                value: "gzip",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "min-size",
                value: "1024",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "selective",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "uri-exclude",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "uri-include",
                value: ".*",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "vary-header",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "HTTPPROXYCONNECT",
        tmsh_kind: "ltm profile http-proxy-connect",
        fields: &[FieldDefault {
            field: "default-state",
            value: "enabled",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "HTTP2",
        tmsh_kind: "ltm profile http2",
        fields: &[
            FieldDefault {
                field: "connection-idle-timeout",
                value: "300",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "insert-header",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "insert-header-name",
                value: "X-HTTP2",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "enforce-tls-requirements",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "include-content-length",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "activation-modes",
                value: "alpn",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "concurrent-streams-per-connection",
                value: "10",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "receive-window",
                value: "32",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "frame-size",
                value: "2048",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "write-size",
                value: "16384",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "header-table-size",
                value: "4096",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "HTTP3",
        tmsh_kind: "ltm profile http3",
        fields: &[FieldDefault {
            field: "header-table-size",
            value: "4096",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "HTTPROUTER",
        tmsh_kind: "ltm profile httprouter",
        fields: &[],
    },
    ProfileDefaults {
        profile: "ICAP",
        tmsh_kind: "ltm profile icap",
        fields: &[],
    },
    ProfileDefaults {
        profile: "ILX",
        tmsh_kind: "ltm profile ilx",
        fields: &[],
    },
    ProfileDefaults {
        profile: "IMAP",
        tmsh_kind: "ltm profile imap",
        fields: &[FieldDefault {
            field: "activation-mode",
            value: "require",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "IPOTHER",
        tmsh_kind: "ltm profile ipother",
        fields: &[FieldDefault {
            field: "idle-timeout",
            value: "60",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "IPSECALG",
        tmsh_kind: "ltm profile ipsecalg",
        fields: &[
            FieldDefault {
                field: "idle-timeout",
                value: "3600",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "pending-ike-connection-limit",
                value: "5",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "initial-connection-timeout",
                value: "3",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "JSON",
        tmsh_kind: "ltm profile json",
        fields: &[
            FieldDefault {
                field: "maximum-bytes",
                value: "65536",
                range: VersionRange::from(BigipVersion::new(21, 0, 0, 0)),
            },
            FieldDefault {
                field: "maximum-entries",
                value: "2048",
                range: VersionRange::from(BigipVersion::new(21, 0, 0, 0)),
            },
            FieldDefault {
                field: "maximum-non-json-bytes",
                value: "32768",
                range: VersionRange::from(BigipVersion::new(21, 0, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "MAPT",
        tmsh_kind: "ltm profile map-t",
        fields: &[
            FieldDefault {
                field: "ip6-prefix",
                value: "::/48",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "ip4-prefix",
                value: "0.0.0.0/8",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "ea-bits-length",
                value: "32",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "port-offset",
                value: "6",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "br-prefix",
                value: "::/96",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "MQTT",
        tmsh_kind: "ltm profile mqtt",
        fields: &[],
    },
    ProfileDefaults {
        profile: "MRRATELIMIT",
        tmsh_kind: "ltm profile mr-ratelimit",
        fields: &[],
    },
    ProfileDefaults {
        profile: "NATSTATS",
        tmsh_kind: "ltm profile nat-stats",
        fields: &[FieldDefault {
            field: "level",
            value: "disabled",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "NETFLOW",
        tmsh_kind: "ltm profile netflow",
        fields: &[],
    },
    ProfileDefaults {
        profile: "OCSP",
        tmsh_kind: "ltm profile ocsp",
        fields: &[],
    },
    ProfileDefaults {
        profile: "ONECONNECT",
        tmsh_kind: "ltm profile one-connect",
        fields: &[
            FieldDefault {
                field: "idle-timeout-override",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "max-age",
                value: "86400",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "max-reuse",
                value: "1000",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "max-size",
                value: "10000",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "source-mask",
                value: "any",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "PCP",
        tmsh_kind: "ltm profile pcp",
        fields: &[
            FieldDefault {
                field: "announce-after-failover",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "announce-multicast",
                value: "10",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "listening-port",
                value: "5351",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "map-filter-limit",
                value: "1",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "map-limit-per-client",
                value: "65535",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "map-recycle-delay",
                value: "60",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "max-mapping-lifetime",
                value: "86400",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "min-mapping-lifetime",
                value: "600",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "multicast-port",
                value: "5350",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "peer-oper-allowed",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "rule",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "third-party-option",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "POP3",
        tmsh_kind: "ltm profile pop3",
        fields: &[FieldDefault {
            field: "activation-mode",
            value: "require",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "PPTP",
        tmsh_kind: "ltm profile pptp",
        fields: &[FieldDefault {
            field: "include-destination-ip",
            value: "disabled",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "QOE",
        tmsh_kind: "ltm profile qoe",
        fields: &[],
    },
    ProfileDefaults {
        profile: "QUIC",
        tmsh_kind: "ltm profile quic",
        fields: &[
            FieldDefault {
                field: "bidi-concurrent-streams-per-connection",
                value: "100",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "uni-concurrent-streams-per-connection",
                value: "10",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "spin-bit",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "RADIUS",
        tmsh_kind: "ltm profile radius",
        fields: &[
            FieldDefault {
                field: "clients",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "persist-avp",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "REQUESTADAPT",
        tmsh_kind: "ltm profile request-adapt",
        fields: &[],
    },
    ProfileDefaults {
        profile: "REQUESTLOG",
        tmsh_kind: "ltm profile request-log",
        fields: &[
            FieldDefault {
                field: "request-logging",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "response-logging",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "RESPONSEADAPT",
        tmsh_kind: "ltm profile response-adapt",
        fields: &[],
    },
    ProfileDefaults {
        profile: "REWRITE",
        tmsh_kind: "ltm profile rewrite",
        fields: &[
            FieldDefault {
                field: "client-caching-type",
                value: "cache-css-js",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "bypass-list",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "rewrite-list",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "split-tunneling",
                value: "false",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "java-ca-file",
                value: "/Common/ca-bundle.crt",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "java-sign-key",
                value: "/Common/default.key",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "java-signer",
                value: "/Common/default.crt",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "RTSP",
        tmsh_kind: "ltm profile rtsp",
        fields: &[FieldDefault {
            field: "idle-timeout",
            value: "300",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "SCTP",
        tmsh_kind: "ltm profile sctp",
        fields: &[
            FieldDefault {
                field: "idle-timeout",
                value: "300",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "init-max-retries",
                value: "8",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "receive-ordered",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "receive-window-size",
                value: "65535",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "reset-on-timeout",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "secret",
                value: "default",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "send-buffer-size",
                value: "65536",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "send-max-retries",
                value: "10",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "send-partial",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "tcp-shutdown",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "SERVERLDAP",
        tmsh_kind: "ltm profile server-ldap",
        fields: &[FieldDefault {
            field: "activation-mode",
            value: "none",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "SERVERSSL",
        tmsh_kind: "ltm profile server-ssl",
        fields: &[
            FieldDefault {
                field: "alert-timeout",
                value: "indefinite",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "authenticate",
                value: "once",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "authenticate-depth",
                value: "9",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "authenticate-name",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "ca-file",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-size",
                value: "262144",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-timeout",
                value: "3600",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "c3d-cert-extension-includes",
                value: "basic-constraints extended-key-usage key-usage subject-alternative-name",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cert",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "chain",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cipher-group",
                value: "none",
                range: VersionRange::between(
                    BigipVersion::new(17, 1, 0, 0),
                    BigipVersion::new(21, 1, 0, 0),
                ),
            },
            FieldDefault {
                field: "cipher-group",
                value: "/Common/f5-default",
                range: VersionRange::from(BigipVersion::new(21, 1, 0, 0)),
            },
            FieldDefault {
                field: "ciphers",
                value: "DEFAULT",
                range: VersionRange::between(
                    BigipVersion::new(17, 1, 0, 0),
                    BigipVersion::new(21, 1, 0, 0),
                ),
            },
            FieldDefault {
                field: "ciphers",
                value: "none",
                range: VersionRange::from(BigipVersion::new(21, 1, 0, 0)),
            },
            FieldDefault {
                field: "crl-file",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "handshake-timeout",
                value: "10",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "key",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "mod-ssl-methods",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "mode",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "options",
                value: "dont-insert-empty-fragments no-tlsv1.3 no-dtlsv1.2",
                range: VersionRange::between(
                    BigipVersion::new(17, 1, 0, 0),
                    BigipVersion::new(21, 1, 0, 0),
                ),
            },
            FieldDefault {
                field: "options",
                value: "dont-insert-empty-fragments no-tlsv1.1 no-tlsv1 no-ssl",
                range: VersionRange::from(BigipVersion::new(21, 1, 0, 0)),
            },
            FieldDefault {
                field: "passphrase",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "peer-cert-mode",
                value: "ignore",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "renegotiate-period",
                value: "indefinite",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "renegotiate-size",
                value: "indefinite",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "renegotiation",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "secure-renegotiation",
                value: "require-strict",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "strict-resume",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "unclean-shutdown",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "log-publisher",
                value: "/Common/sys-ssl-publisher",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "SERVICE",
        tmsh_kind: "ltm profile service",
        fields: &[],
    },
    ProfileDefaults {
        profile: "SIP",
        tmsh_kind: "ltm profile sip",
        fields: &[
            FieldDefault {
                field: "insert-record-route-header",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "insert-via-header",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "max-size",
                value: "65535",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "secure-via-header",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "terminate-on-bye",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "SMTPS",
        tmsh_kind: "ltm profile smtps",
        fields: &[FieldDefault {
            field: "activation-mode",
            value: "require",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "SOCKS",
        tmsh_kind: "ltm profile socks",
        fields: &[
            FieldDefault {
                field: "protocol-versions",
                value: "socks4 socks4a socks5",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "default-connect-handling",
                value: "deny",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "ipv6",
                value: "no",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "route-domain",
                value: "/Common/0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "tunnel-name",
                value: "/Common/socks-tunnel",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "SPLITSESSIONCLIENT",
        tmsh_kind: "ltm profile splitsessionclient",
        fields: &[],
    },
    ProfileDefaults {
        profile: "SPLITSESSIONSERVER",
        tmsh_kind: "ltm profile splitsessionserver",
        fields: &[],
    },
    ProfileDefaults {
        profile: "SSE",
        tmsh_kind: "ltm profile sse",
        fields: &[
            FieldDefault {
                field: "max-buffered-msg-bytes",
                value: "65536",
                range: VersionRange::from(BigipVersion::new(21, 0, 0, 0)),
            },
            FieldDefault {
                field: "max-field-name-size",
                value: "1024",
                range: VersionRange::from(BigipVersion::new(21, 0, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "STATISTICS",
        tmsh_kind: "ltm profile statistics",
        fields: &[
            FieldDefault {
                field: "field1",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field2",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field3",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field4",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field5",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field6",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field7",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field8",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field9",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field10",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field11",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field12",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field13",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field14",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field15",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field16",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field17",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field18",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field19",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field20",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field21",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field22",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field23",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field24",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field25",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field26",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field27",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field28",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field29",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field30",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field31",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "field32",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "STREAM",
        tmsh_kind: "ltm profile stream",
        fields: &[
            FieldDefault {
                field: "source",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "target",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "TCP",
        tmsh_kind: "ltm profile tcp",
        fields: &[
            FieldDefault {
                field: "abc",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "ack-on-push",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "auto-proxy-buffer-size",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "auto-receive-window-size",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "auto-send-buffer-size",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "close-wait-timeout",
                value: "5",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cmetrics-cache",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cmetrics-cache-timeout",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "congestion-control",
                value: "high-speed",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "deferred-accept",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "delayed-acks",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "delay-window-control",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "dsack",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "early-retransmit",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "ecn",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "enhanced-loss-recovery",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "fast-open",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "fast-open-cookie-expiration",
                value: "21600",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "fin-wait-timeout",
                value: "5",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "fin-wait-2-timeout",
                value: "300",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "idle-timeout",
                value: "300",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "init-cwnd",
                value: "10",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "init-rwnd",
                value: "10",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "ip-tos-to-client",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "keep-alive-interval",
                value: "1800",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "limited-transmit",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "link-qos-to-client",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "max-retrans",
                value: "8",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "max-segment-size",
                value: "1460",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "md5-signature",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "minimum-rto",
                value: "1000",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "mptcp",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "nagle",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "pkt-loss-ignore-burst",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "pkt-loss-ignore-rate",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "proxy-buffer-high",
                value: "65535",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "proxy-buffer-low",
                value: "32768",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "proxy-mss",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "proxy-options",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "push-flag",
                value: "default",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "rate-pace",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "rate-pace-max-rate",
                value: "0",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "receive-window-size",
                value: "65535",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "reset-on-timeout",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "rexmt-thresh",
                value: "3",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "selective-acks",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "selective-nack",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "send-buffer-size",
                value: "131072",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "slow-start",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "syn-cookie-enable",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "syn-cookie-whitelist",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "syn-max-retrans",
                value: "3",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "syn-rto-base",
                value: "3000",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "tail-loss-probe",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "time-wait-recycle",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "time-wait-timeout",
                value: "2000",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "timestamps",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "verified-accept",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "zero-window-timeout",
                value: "20000",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "TCPANALYTICS",
        tmsh_kind: "ltm profile tcp-analytics",
        fields: &[
            FieldDefault {
                field: "collected-by-client-side",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collected-by-server-side",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collected-stats-internal-logging",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-nexthop",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-continent",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-region",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-city",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-post-code",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-remote-host-ip",
                value: "disabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-remote-host-subnet",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "collect-country",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "TDR",
        tmsh_kind: "ltm profile tdr",
        fields: &[
            FieldDefault {
                field: "filters",
                value: "{ base_filter { tdr-format $:T,$:F,$:L } }",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "log-publisher",
                value: "/Common/local-syslog-publisher",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "TFTP",
        tmsh_kind: "ltm profile tftp",
        fields: &[],
    },
    ProfileDefaults {
        profile: "TRAFFICACCELERATION",
        tmsh_kind: "ltm profile traffic-acceleration",
        fields: &[],
    },
    ProfileDefaults {
        profile: "UDP",
        tmsh_kind: "ltm profile udp",
        fields: &[FieldDefault {
            field: "idle-timeout",
            value: "60",
            range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
        }],
    },
    ProfileDefaults {
        profile: "WEBACCELERATION",
        tmsh_kind: "ltm profile web-acceleration",
        fields: &[
            FieldDefault {
                field: "cache-aging-rate",
                value: "9",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-client-cache-control-mode",
                value: "all",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-insert-age-header",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-max-age",
                value: "3600",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-max-entries",
                value: "10000",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-object-max-size",
                value: "50000",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-object-min-size",
                value: "500",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-size",
                value: "100",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-uri-include",
                value: ".*",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-uri-include-override",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-uri-exclude",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "cache-uri-pinned",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "WEBSECURITY",
        tmsh_kind: "ltm profile web-security",
        fields: &[],
    },
    ProfileDefaults {
        profile: "WEBSOCKET",
        tmsh_kind: "ltm profile websocket",
        fields: &[
            FieldDefault {
                field: "masking",
                value: "selective",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "compress-mode",
                value: "preserved",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "compression",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "window-bits",
                value: "10",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "no-delay",
                value: "enabled",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "payload-processing-mode",
                value: "end-to-end",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
            FieldDefault {
                field: "payload-protocol-profile",
                value: "none",
                range: VersionRange::from(BigipVersion::new(17, 1, 0, 0)),
            },
        ],
    },
    ProfileDefaults {
        profile: "XML",
        tmsh_kind: "ltm profile xml",
        fields: &[],
    },
];
