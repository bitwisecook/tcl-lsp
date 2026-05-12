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

  program       := statement (';' statement)*
  statement     := assignment | pipeline
  assignment    := path ('=' | '|=' | '+=' | '-=') pipeline
  pipeline      := or_expr ('|' or_expr)*
  or_expr       := and_expr ('or' and_expr)*
  and_expr      := not_expr ('and' not_expr)*
  not_expr      := 'not' not_expr | cmp_expr
  cmp_expr      := add_expr (('==' | '!=' | '<' | '<=' | '>' | '>=') add_expr)?
  add_expr      := mul_expr (('+' | '-') mul_expr)*
  mul_expr      := unary    (('*' | '/') unary)*
  unary         := '-' unary | postfix
  postfix       := primary
  primary       := literal | call | path | '(' pipeline ')'
  path          := '.'
                 | '.' field path_tail
                 | '.' '[' subscript ']' path_tail
  path_tail     := ('.' field | '[' subscript ']')*
  field         := IDENT | STRING
  subscript     := /* empty -> stream */
                 | NUMBER
                 | STRING                /* exact subscript by string  */
                 | REGEX                 /* ["~pattern"] — regex match */
                 | pipeline              /* dynamic subscript          */
  call          := IDENT '(' (pipeline (',' pipeline)*)? ')'
  literal       := NUMBER | STRING | 'true' | 'false' | 'null'

PATH ACCESS

  .                       The current value (identity).
  .ltm.virtual            Field access — chained.
  .ltm.virtual.web_vs     TMSH partition shorthand: bare names resolve to
                          ``/Common/web_vs`` when unambiguous in the
                          target container.  Quote the full path to be
                          explicit: ``."[/Common/web_vs]"``.
  .ltm.virtual["/Common/web_vs"]
                          Exact subscript by full-path.
  .ltm.virtual["~^vs_prod_"]
                          Regex subscript — matches every key whose
                          full-path matches the pattern.
  .ltm.virtual[]          Stream every value in the container.
  .ltm.virtual[].pool     A path-ref to the default pool of each VS.
                          PathRefs act as strings AND as the referenced
                          object: ``.ltm.virtual[].pool.members[]`` walks
                          VS -> pool -> member transparently.

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

DIFFERENCES FROM jq

  * Function arguments are comma-separated, not semicolon-separated.
    ``sub(.name, "foo", "bar")`` rather than ``sub("foo"; "bar")``.
  * No stream-comma operator — combine streams with explicit lists or
    repeated statements separated by ``;``.
  * Identifiers may contain ``-`` so TMSH-spelt keys like
    ``data-group`` and ``source-address-translation`` are bareword
    tokens; you still need quotes when a hyphen would otherwise be
    parsed as subtraction (``."source-address-translation"``).
  * Regex matching has a dedicated subscript form ``["~pattern"]``
    rather than the ``test`` builtin.

See also:
  --help-builtins      every function this DSL exposes
  --help-examples      a cookbook of common one-liners
"""


def format_grammar() -> str:
    """Return the DSL grammar reference as a single string."""
    return _GRAMMAR
