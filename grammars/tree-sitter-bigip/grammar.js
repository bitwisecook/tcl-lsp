/**
 * tree-sitter grammar for an F5 BIG-IP configuration (`bigip.conf` / `.scf`).
 *
 * A BIG-IP config is a brace-structured object language, not Tcl — but an
 * `ltm rule` body *is* Tcl (an iRule), and `injections.scm` re-parses it as
 * such, exactly as the tcl-lsp server walks it with the iRules dialect.
 *
 * Written for tcl-lsp (issue #903). Zed pointed its TMSH language at the *Tcl*
 * grammar, which produces a garbage parse tree for a file that is not Tcl.
 *
 * Two decisions worth knowing:
 *
 * 1. **Newlines are significant.** The format is line-oriented: a property ends
 *    at the end of its line unless a `{` opens a block. With `\n` in `extras`
 *    the grammar is genuinely ambiguous — nothing distinguishes `pool /Common/a`
 *    followed by a new property from `pool` taking two values — and tree-sitter
 *    rightly refuses to generate.
 *
 * 2. **The iRule body is an external token** (`src/scanner.c`). Modelling it as
 *    balanced-brace token soup inside the grammar works until a brace appears
 *    behind a backslash, at which point the body node ends in the wrong place
 *    and the injected Tcl is truncated.
 *
 * @file BIG-IP configuration grammar for tree-sitter
 * @author tcl-lsp contributors
 * @license AGPL-3.0-or-later
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

/** The tmsh top-level modules. */
const MODULES = [
  'analytics',
  'apm',
  'asm',
  'auth',
  'cli',
  'cm',
  'gtm',
  'ilx',
  'ltm',
  'net',
  'pem',
  'security',
  'sys',
  'vcmp',
  'wom',
];

module.exports = grammar({
  name: 'bigip',

  // Newlines are NOT extras — see the note above.
  extras: ($) => [/[ \t\r]/, $.comment],

  externals: ($) => [$.rule_body],

  rules: {
    source_file: ($) => repeat(choice($._newline, $._declaration)),

    _newline: (_) => token(/\n/),

    _declaration: ($) => choice($.irule, $.object),

    comment: (_) => token(seq('#', /[^\n]*/)),

    // `ltm rule /Common/name { …Tcl… }`.
    //
    // `rule` carries a higher token precedence than the generic `object_type`
    // regex, which also matches it — without that, `ltm rule` lexes as an
    // ordinary object and the body is parsed as config rather than Tcl. The
    // precedence is safe because tree-sitter only lexes a token where the
    // grammar says it is valid: `rule` as a property *value* inside a block is
    // still an ordinary word.
    irule: ($) =>
      seq(
        field('module', $.module),
        field('type', alias($._rule_keyword, $.object_type)),
        field('name', $.object_path),
        '{',
        optional(field('body', $.rule_body)),
        '}',
      ),

    _rule_keyword: (_) => token(prec(2, 'rule')),

    // `<module> <type…> <name>? { … }` — e.g. `ltm data-group internal /Common/x {`
    object: ($) =>
      seq(
        field('module', $.module),
        repeat1(field('type', $.object_type)),
        optional(field('name', $.object_path)),
        field('body', $.block),
      ),

    module: (_) => token(prec(2, choice(...MODULES))),

    object_type: (_) => token(prec(1, /[a-z][a-z0-9_-]*/)),

    // Each property is newline-terminated. Without that terminator, `pool
    // /Common/a` is ambiguous — one property taking two values, or two
    // properties abutting — and the grammar will not generate. The final
    // property before `}` may omit it, for the `records { www.example.com { } }`
    // shape tmsh emits.
    block: ($) =>
      seq(
        '{',
        repeat($._newline),
        repeat(seq($.property, repeat1($._newline))),
        optional($.property),
        '}',
      ),

    // `name value…` — or `name { … }` for a nested block, or a bare
    // `/Common/10.0.0.1:80 { … }` member. A property ends at its newline, which
    // is what makes the grammar unambiguous.
    property: ($) =>
      seq(
        field('name', $._value),
        repeat(field('value', $._value)),
        optional(field('body', $.block)),
      ),

    _value: ($) =>
      choice($.object_path, $.ip_address, $.encrypted, $.string, $.number, $.word),

    // `/Partition/name`, with optional folder components, route domain (`%1`)
    // and port (`:80`). A member's final component *is* an address; rather than
    // complicate the grammar, `highlights.scm` matches its text and colours it
    // as one — so the grammar never has to guess.
    // Every separator is `token.immediate`: whitespace is an extra, so without
    // it `10.0.0.0/8` (a CIDR) and `10.0.0.1 /Common/x` (an address followed by
    // a path) are indistinguishable to the parser, and tree-sitter rightly
    // refuses to generate.
    object_path: ($) =>
      choice(
        // `/Partition/name`, with optional extra folder components.
        seq(
          '/',
          field('partition', $.path_component),
          repeat1(seq(token.immediate('/'), field('name', $.path_component))),
          optional(seq(token.immediate('%'), field('route_domain', $.number))),
          optional(seq(token.immediate(':'), field('port', $.port))),
        ),
        // A single-component path — a data-group record key such as
        // `/old-path` is a URI, not a partitioned object, and has no partition
        // to name. Requiring two components here made every data-group in the
        // sample config fail to parse.
        seq(
          '/',
          field('name', $.path_component),
          optional(seq(token.immediate('%'), field('route_domain', $.number))),
          optional(seq(token.immediate(':'), field('port', $.port))),
        ),
      ),

    path_component: (_) => token.immediate(/[^/\s{}:%]+/),

    port: (_) => token.immediate(/[0-9]+|[a-z][a-z0-9-]*/),

    ip_address: ($) =>
      seq(
        $._ip_literal,
        optional(seq(token.immediate('%'), field('route_domain', $.number))),
        optional(seq(token.immediate('/'), field('prefix', $.number))),
        optional(seq(token.immediate(':'), field('port', $.port))),
      ),

    _ip_literal: (_) =>
      token(
        prec(
          3,
          choice(
            /[0-9]{1,3}(\.[0-9]{1,3}){3}/,
            /[0-9a-fA-F]{0,4}(:[0-9a-fA-F]{0,4}){2,7}/,
          ),
        ),
      ),

    // tmsh writes secrets as `$M$…`.
    encrypted: (_) => token(prec(3, /\$[A-Za-z0-9]\$[^\s{}"]+/)),

    string: ($) => seq('"', repeat(choice($.escape_sequence, /[^"\\\n]+/)), '"'),

    escape_sequence: (_) =>
      token(
        seq(
          '\\',
          choice(
            /x[0-9a-fA-F]{1,2}/,
            /u[0-9a-fA-F]{1,4}/,
            /U[0-9a-fA-F]{1,8}/,
            /[0-7]{1,3}/,
            /./,
          ),
        ),
      ),

    number: (_) => token(prec(2, /[+-]?[0-9]+(\.[0-9]+)?/)),

    _immediate_number: (_) => token.immediate(/[0-9]+/),

    // A bare word. It may not begin with `/`, or it would swallow an
    // `object_path` whole (tree-sitter takes the longest match, and a word
    // spanning `/Common/api_pool` is longer than the path's leading `/`).
    word: (_) => token(/[^\s{}"/][^\s{}"]*/),
  },
});
