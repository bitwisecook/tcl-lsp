"""Plain-text grammar reference rendered by ``f5 query --help-dsl``.

The canonical reference lives in ``docs/design/f5-query-dsl.md``; this
file holds the condensed terminal-friendly version so users on an
air-gapped device can read the grammar without leaving the shell.

The grammar below is informal EBNF.  Where it deviates from jq we say
so inline.
"""

from __future__ import annotations

_GRAMMAR = r"""F5 QUERY DSL — GRAMMAR

A query is a pipeline of stages joined by ``|``.  Each stage transforms
the current value (``.``) into one or more new values.  Pipelines may
be chained with ``;`` to evaluate multiple statements against the same
root.

  program       := pipeline (';' pipeline)*
  pipeline      := pipe_stage ('|' pipe_stage)*
  pipe_stage    := or_expr (ASSIGN_OP pipe_stage)?
  ASSIGN_OP     := '=' | '|=' | '+=' | '-='
  or_expr       := and_expr ('or' and_expr)*
  and_expr      := not_expr ('and' not_expr)*
  not_expr      := 'not' not_expr | cmp_expr
  cmp_expr      := add_expr (('==' | '!=' | '<' | '<=' | '>' | '>=') add_expr)?
  add_expr      := mul_expr (('+' | '-') mul_expr)*
  mul_expr      := unary    (('*' | '/') unary)*
  unary         := '-' unary | postfix
  postfix       := primary path_tail
  primary       := literal | call | path | variable | list_literal
                 | '(' pipeline ')'
  variable      := '$' IDENT          /* root container of a named
                                         source loaded alongside this
                                         query (`f5 query` accepts
                                         several configs with
                                         filename-stem default names
                                         and an explicit --name N=PATH
                                         override).  Postfix path
                                         steps land afterwards, so
                                         `$gtm.gtm.wideip[]` reads from
                                         the source bound under
                                         `$gtm`.                       */
  list_literal  := '[' pipeline? ']'        /* jq's array constructor */
  path          := '.'
                 | '.' field path_tail
                 | '.' '[' subscript ']' path_tail
  path_tail     := ('.' field | '[' subscript ']')*
  field         := IDENT | STRING
  subscript     := /* empty -> stream */
                 | NUMBER
                 | STRING                /* exact subscript by string  */
                                         /* STRING starting with "~"
                                            inside [ ] is the regex
                                            subscript form            */
                 | pipeline              /* dynamic subscript          */
  call          := IDENT ('(' (pipeline (',' pipeline)*)? ')')?
                                         /* parens optional for 1-arg
                                            builtins — bare ``length``
                                            is equivalent to
                                            ``length(.)``               */
  literal       := NUMBER | STRING | 'true' | 'false' | 'null'

Pipelines iterate **streams**, not plain lists.  ``[]`` produces a
stream and stream-returning builtins do too; ``|`` then runs the next
stage once per item.  Plain lists (e.g. the value of ``.rules``) pass
through ``|`` whole — to fold a stream into a list for aggregators
like ``sort`` / ``unique``, wrap it: ``[.ltm.virtual[].name] | sort``.

Assignment is a *trailing operator* on a pipe-stage, not a top-level
statement.  That makes ``.ltm.virtual[] | .destination |= ip(...)``
parse as ``.ltm.virtual[] | (.destination |= ip(...))`` — for each
streamed VS, set its destination to ``ip(...)`` of the current value.

PATH ACCESS

  .                       The current value (identity).
  .ltm.virtual            Field access — chained.
  .ltm.virtual.web_vs     TMSH partition shorthand: bare names resolve to
                          ``/Common/web_vs`` when unambiguous in the
                          target container.  Use the subscript form
                          ``.ltm.virtual["/Common/web_vs"]`` (see below)
                          for an explicit full-path lookup.
  .ltm.virtual["/Common/web_vs"]
                          Exact subscript by full-path.
  .ltm.virtual["~/vs_prod_"]
                          Regex subscript — matches every key whose
                          full-path (including the partition prefix)
                          contains the pattern.  Use the explicit
                          partition prefix in the pattern when you
                          want to anchor the match
                          (``"~^/Common/vs_prod_"``).
  .ltm.virtual[]          Stream every value in the container.
  .ltm.virtual[].pool     A path-ref to the default pool of each VS.
                          PathRefs act as strings AND as the referenced
                          object: ``.ltm.virtual[].pool.members[]`` walks
                          VS -> pool -> member transparently.

MODULES

  .ltm.<kind>             Local Traffic Manager kinds: ``virtual``,
                          ``pool``, ``node``, ``rule``, ``profile``,
                          ``monitor``, ``persistence``, ``snatpool``,
                          ``policy``, ``data-group``.  Each
                          ``ltm policy`` exposes ``.rules[]`` with
                          nested ``.conditions[]`` and
                          ``.actions[]`` sub-objects — chained
                          access like ``.ltm.policy[].rules[]
                          .actions[].pool`` walks into the target
                          pool via a PathRef.
  .net.<kind>             Network module: ``route``, ``vlan``,
                          ``self``, ``route-domain``, ``port-list``,
                          ``interface``, ``dns-resolver``,
                          ``tunnels-tunnel``, ``stp``.
                          PathRefs from ``net self.vlan`` and
                          ``net route-domain.vlans[]`` auto-deref
                          into the target ``net vlan`` so chained
                          access ``.net.self[].vlan.tag`` Just Works.

  .sys.<kind>             System module: ``dns``, ``ntp``, ``snmp``,
                          ``global-settings`` (singletons — one entry
                          each, streamed via ``.sys.dns[]``);
                          ``provision``, ``folder``, ``file-ssl-cert``,
                          ``file-ssl-key``, ``management-route``.
  .security.<kind>        Security module:
                          ``firewall-port-list``,
                          ``firewall-rule-list``,
                          ``firewall-config-entity-id``,
                          ``ip-intelligence-policy``,
                          ``protocol-inspection-compliance-map``,
                          ``protocol-inspection-compliance-objects``,
                          ``device-id-attribute``.
  .apm.<kind>             Access Policy Manager: ``access-policy``,
                          ``policy-item``, ``policy-agent``,
                          ``customization-source``,
                          ``oauth-db-instance``,
                          ``ssh-security-config``,
                          ``default-report`` (singleton).
                          PathRefs from ``access-policy.items[]``
                          and ``access-policy.start-item`` auto-deref
                          into ``policy-item`` so ``.apm.access-policy
                          [].start-item.caption`` Just Works.
  .cm.<kind>              Cluster Manager: ``cert``, ``key``,
                          ``device``, ``device-group``,
                          ``traffic-group``, ``trust-domain``.
                          PathRefs from ``device.cert`` /
                          ``device.key`` and ``trust-domain.ca-cert``
                          / ``ca-key`` / ``ca-devices[]`` /
                          ``trust-group`` auto-deref so
                          ``.cm.trust-domain[].trust-group.devices``
                          walks ``trust-domain → device-group →
                          devices`` in one chain.
  .gtm.<kind>             Global Traffic Manager (DNS):
                          ``datacenter``, ``server``, ``pool``,
                          ``wideip``, ``prober-pool``, ``region``,
                          ``rule``.  ``pool`` and ``wideip`` merge
                          all DNS record types (``a / aaaa / cname /
                          mx / srv / naptr``) into one container; the
                          DNS record type is exposed as
                          ``.record-type``.  PathRefs from
                          ``server.datacenter``,
                          ``wideip.pools[]`` / ``last-resort-pool``,
                          and ``prober-pool.members[]`` auto-deref
                          so chained queries like
                          ``.gtm.wideip[].pools[].ttl`` walk
                          ``wideip → pool → ttl`` in one step.
  .pem.<kind>             Policy Enforcement Manager: ``policy``,
                          ``irule``, ``listener``,
                          ``forwarding-endpoint``,
                          ``interception-endpoint``,
                          ``service-chain-endpoint``, ``profile``,
                          ``rating-group``, plus ``global-settings-*``,
                          ``protocol-*``, ``reporting-format-script``,
                          ``subscriber``, ``subscriber-attribute``.
  .auth.<kind>            Authentication: ``partition``, ``user``,
                          ``ldap``, ``radius``, ``radius-server``,
                          ``tacacs``, ``apm-auth``, ``cert-ldap``,
                          plus singletons (``password``,
                          ``password-policy``, ``source``,
                          ``remote-role``, ``remote-user``,
                          ``login-failures``).
  .vcmp.<kind>            vCMP: ``guest``, ``traffic-profile``,
                          ``virtual-disk``, ``virtual-disk-template``.
  .cli.<kind>             CLI: ``admin-partitions``,
                          ``alias-private``, ``alias-shared``,
                          ``global-settings``, ``preference``,
                          ``script``, ``transaction``, ``version``.
  .api-protection.<kind>  API Protection: ``profile-apiprotection``,
                          ``response``, ``server``.
  .asm.<kind>             Application Security Manager: ``policy``.
  .ilx.<kind>             iRulesLX: ``global-settings`` (singleton).
  .wom.<kind>             WAN Optimization Manager (legacy):
                          ``endpoint-discovery`` (singleton).
  .analytics.<kind>       Analytics: ``global-settings`` (singleton).

  Any TMSH stanza the parser sees but no typed projection covers
  still lands in ``cfg.generic_objects`` with full byte ranges
  and is reachable via ``rename_partition`` / ``--scf`` source-
  level operations, but it is not navigable from the DSL.

ASSIGNMENT

  path = expr             Set the target field to ``expr`` (evaluated
                          against the outer input).
  path |= expr            "Update": set the target to ``path | expr``;
                          ``.`` inside expr is the current value.
  path += expr            Numeric add, string concat, or list append.
  path -= expr            Numeric sub, or remove items from a list.

Assigning to an object's identity field (``.name`` / ``."full-path"``)
auto-routes through the same engine ``f5 rename`` uses, rewriting
every reference to the object as well as its header.  A line like
``renamed X -> Y (N occurrences)`` is printed to stderr so the
multi-stanza rewrite is visible.

Writing into an iRule body in v1 is restricted to reference slots
(``.refs.pools[]`` etc.).  General command-argument rewriting is
deferred to v2.

OPERATORS AND PRECEDENCE

  Highest -> lowest:
    1. unary  '-', 'not'
    2. '*' '/'
    3. '+' '-'
    4. '==' '!=' '<' '<=' '>' '>='
    5. 'and'
    6. 'or'
    7. '|' (pipe)
    8. '=' '|=' '+=' '-=' (assignment)
    9. ';' (statement separator)

JQ COMPATIBILITY

  Core idioms match jq exactly:
    * ``.X[]`` is a stream-generator; ``|`` iterates it.
    * ``[ ... ]`` collects a stream into an array.
    * Bare builtin names (``length``, ``sort``, ``unique``) operate
      on ``.`` — equivalent to ``length(.)`` etc.
    * Plain lists pass through ``|`` whole — to iterate a list, use
      ``.[]`` (or pass it to a list-aware builtin like ``map``).

  Differences:
    * Function arguments are comma-separated, not semicolon-separated.
      ``sub(.name, "foo", "bar")`` rather than ``sub("foo"; "bar")``.
    * No stream-comma operator — use ``;`` between statements (each
      runs against the evolving source) or wrap with a list literal.
    * Identifiers may contain ``-`` so TMSH-spelt keys like
      ``data-group`` and ``source-address-translation`` lex as a
      single bareword token — ``.source-address-translation`` parses
      as one field access, no quotes needed.  Quoting is only useful
      when the key would otherwise tokenise into something else
      (e.g. ``."full-path"`` so the ``.`` after ``full`` is not read
      as a field-access separator).
    * Regex matching has a dedicated subscript form ``["~pattern"]``
      rather than the ``test`` builtin.
    * No object-literal ``{ ... }`` or string-interpolation ``"\(.x)"``
      in v1 — use ``--json`` to render whole objects as structured
      output.

See also:
  --help-builtins      every function this DSL exposes
  --help-examples      a cookbook of common one-liners
"""


def format_grammar() -> str:
    """Return the DSL grammar reference as a single string."""
    return _GRAMMAR
