"""Tests for the ``ltm policy`` parser additions."""

from __future__ import annotations

from core.bigip.parser import parse_bigip_conf


def test_policy_minimal():
    source = """\
ltm policy /Common/p_min {
    rules {
        only {
            ordinal 1
            actions {
                0 {
                    forward
                    select
                    pool /Common/p_default
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    assert "/Common/p_min" in cfg.policies
    p = cfg.policies["/Common/p_min"]
    assert p.name == "p_min"
    assert p.strategy == "first-match"
    assert len(p.rules) == 1
    rule = p.rules[0]
    assert rule.name == "only"
    assert rule.ordinal == 1
    assert rule.conditions == ()
    assert len(rule.actions) == 1
    act = rule.actions[0]
    assert act.target == "forward"
    assert act.verb == "select"
    assert act.pool == "/Common/p_default"


def test_policy_strips_strategy_path():
    source = """\
ltm policy /Common/p_strat {
    rules { only { ordinal 1 } }
    strategy /Common/best-match
}
"""
    cfg = parse_bigip_conf(source)
    assert cfg.policies["/Common/p_strat"].strategy == "best-match"


def test_policy_http_host_condition():
    source = """\
ltm policy /Common/p1 {
    rules {
        host_eq {
            ordinal 1
            conditions {
                0 {
                    http-host
                    host
                    equals
                    values { api.example.com www.example.com }
                    request
                }
            }
            actions {
                0 {
                    forward
                    select
                    pool /Common/p_api
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    rule = cfg.policies["/Common/p1"].rules[0]
    cond = rule.conditions[0]
    assert cond.operand == "http-host"
    assert cond.selector == "host"
    assert cond.operator == "equals"
    assert cond.values == ("api.example.com", "www.example.com")
    assert cond.event == "request"
    assert not cond.negate


def test_policy_http_uri_path_starts_with():
    source = """\
ltm policy /Common/p2 {
    rules {
        api_path {
            ordinal 1
            conditions {
                0 {
                    http-uri
                    path
                    starts-with
                    case-insensitive
                    values { /api/ /v1/ }
                    request
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    cond = cfg.policies["/Common/p2"].rules[0].conditions[0]
    assert cond.operand == "http-uri"
    assert cond.selector == "path"
    assert cond.operator == "starts-with"
    assert cond.case_insensitive is True
    assert cond.values == ("/api/", "/v1/")


def test_policy_negated_condition():
    source = """\
ltm policy /Common/p3 {
    rules {
        not_internal {
            ordinal 1
            conditions {
                0 {
                    http-uri
                    path
                    starts-with
                    not
                    values { /internal/ }
                    request
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    cond = cfg.policies["/Common/p3"].rules[0].conditions[0]
    assert cond.negate is True
    assert cond.operator == "starts-with"


def test_policy_http_method_condition():
    source = """\
ltm policy /Common/p4 {
    rules {
        only_get {
            ordinal 1
            conditions {
                0 {
                    http-method
                    values { GET HEAD }
                    request
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    cond = cfg.policies["/Common/p4"].rules[0].conditions[0]
    assert cond.operand == "http-method"
    assert cond.values == ("GET", "HEAD")


def test_policy_http_header_condition_with_name():
    source = """\
ltm policy /Common/p5 {
    rules {
        hdr_match {
            ordinal 1
            conditions {
                0 {
                    http-header
                    name X-Forwarded-Proto
                    values { https }
                    request
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    cond = cfg.policies["/Common/p5"].rules[0].conditions[0]
    assert cond.operand == "http-header"
    assert cond.name == "X-Forwarded-Proto"
    assert cond.values == ("https",)


def test_policy_ssl_extension_sni_condition():
    source = """\
ltm policy /Common/p6 {
    rules {
        sni_match {
            ordinal 1
            conditions {
                0 {
                    ssl-extension
                    name server-name
                    values { api.example.com }
                    ssl-client-hello
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    cond = cfg.policies["/Common/p6"].rules[0].conditions[0]
    assert cond.operand == "ssl-extension"
    assert cond.name == "server-name"
    assert cond.event == "ssl-client-hello"


def test_policy_http_reply_redirect_action():
    source = """\
ltm policy /Common/p7 {
    rules {
        redir {
            ordinal 1
            actions {
                0 {
                    http-reply
                    redirect
                    location "https://www.example.com/welcome"
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    act = cfg.policies["/Common/p7"].rules[0].actions[0]
    assert act.target == "http-reply"
    assert act.verb == "redirect"
    assert act.location == "https://www.example.com/welcome"


def test_policy_http_uri_replace_path_action():
    source = """\
ltm policy /Common/p8 {
    rules {
        rewrite {
            ordinal 1
            actions {
                0 {
                    http-uri
                    replace
                    path "/v2/health"
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    act = cfg.policies["/Common/p8"].rules[0].actions[0]
    assert act.target == "http-uri"
    assert act.verb == "replace"
    assert act.path == "/v2/health"


def test_policy_http_header_insert_action():
    source = """\
ltm policy /Common/p9 {
    rules {
        ins {
            ordinal 1
            actions {
                0 {
                    http-header
                    insert
                    tm-name X-Forwarded-For
                    value "client-ip"
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    act = cfg.policies["/Common/p9"].rules[0].actions[0]
    assert act.target == "http-header"
    assert act.verb == "insert"
    assert act.name == "X-Forwarded-For"
    assert act.value == "client-ip"


def test_policy_http_header_remove_action():
    source = """\
ltm policy /Common/p10 {
    rules {
        rm {
            ordinal 1
            actions {
                0 {
                    http-header
                    remove
                    name Server
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    act = cfg.policies["/Common/p10"].rules[0].actions[0]
    assert act.target == "http-header"
    assert act.verb == "remove"
    assert act.name == "Server"


def test_policy_tcp_reset_action():
    source = """\
ltm policy /Common/p11 {
    rules {
        rst {
            ordinal 1
            actions {
                0 {
                    tcp
                    reset
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    act = cfg.policies["/Common/p11"].rules[0].actions[0]
    assert act.target == "tcp"
    assert act.verb == "reset"


def test_policy_rules_sorted_by_ordinal():
    source = """\
ltm policy /Common/p12 {
    rules {
        third  { ordinal 30 }
        first  { ordinal 10 }
        second { ordinal 20 }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    names = [r.name for r in cfg.policies["/Common/p12"].rules]
    assert names == ["first", "second", "third"]


def test_policy_multiple_conditions_and_actions_preserve_index_order():
    source = """\
ltm policy /Common/p13 {
    rules {
        multi {
            ordinal 1
            conditions {
                1 {
                    http-uri
                    path
                    starts-with
                    values { /v1/ }
                    request
                }
                0 {
                    http-host
                    host
                    equals
                    values { api.example.com }
                    request
                }
            }
            actions {
                1 {
                    http-header
                    insert
                    name X-Trace
                    value "yes"
                }
                0 {
                    forward
                    select
                    pool /Common/pool_api
                }
            }
        }
    }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    rule = cfg.policies["/Common/p13"].rules[0]
    assert [c.index for c in rule.conditions] == [0, 1]
    assert rule.conditions[0].operand == "http-host"
    assert rule.conditions[1].operand == "http-uri"
    assert [a.index for a in rule.actions] == [0, 1]
    assert rule.actions[0].target == "forward"
    assert rule.actions[1].target == "http-header"


def test_policy_strategy_default_when_missing():
    source = """\
ltm policy /Common/p14 {
    rules { r { ordinal 1 } }
}
"""
    cfg = parse_bigip_conf(source)
    assert cfg.policies["/Common/p14"].strategy == "first-match"


def test_policy_does_not_collide_with_policy_strategy_stanza():
    """`ltm policy-strategy` is a different stanza type and must not match."""
    source = """\
ltm policy-strategy /Common/best-match {
    operands {
        0 { http-host }
    }
}
ltm policy /Common/p15 {
    rules { r { ordinal 1 } }
    strategy first-match
}
"""
    cfg = parse_bigip_conf(source)
    assert "/Common/p15" in cfg.policies
    assert "/Common/best-match" not in cfg.policies
