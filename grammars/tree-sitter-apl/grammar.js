/**
 * tree-sitter grammar for F5 iApp APL (Application Presentation Language).
 *
 * APL is the presentation half of an iApp template: it declares the form the
 * BIG-IP admin fills in. It is *not* Tcl — it has its own grammar of sections,
 * typed fields, attributes and choice lists — but it may embed Tcl inside
 * `[ … ]` bracket expressions, which `injections.scm` re-parses as Tcl.
 *
 * Written for tcl-lsp (issue #903). Zed pointed its APL language at the *Tcl*
 * grammar, which produces a garbage parse tree for a file that is not Tcl.
 *
 * @file APL grammar for tree-sitter
 * @author tcl-lsp contributors
 * @license AGPL-3.0-or-later
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

/**
 * The field types APL ships with. A field's type may also be *user-defined*
 * (`define choice lb_method` … then `lb_method lb_algorithm`), so the grammar
 * accepts any identifier here and leaves it to `highlights.scm` to colour the
 * built-in ones — the same split the Tcl queries use for built-in commands.
 */
const BUILTIN_FIELD_TYPES = [
  'addrport',
  'choice',
  'disena',
  'editchoice',
  'enadis',
  'enadisdry',
  'falsetrue',
  'indefint',
  'message',
  'multichoice',
  'noyes',
  'password',
  'string',
  'tcpprof',
  'truefalse',
  'yesno',
];

module.exports = grammar({
  name: 'apl',

  extras: ($) => [/\s/, $.comment],

  word: ($) => $.identifier,

  rules: {
    source_file: ($) => repeat($._declaration),

    _declaration: ($) =>
      choice(
        $.include,
        $.define,
        $.section,
        $.text_block,
        $.table,
        $.message_field,
        $.field,
      ),

    // `#include "f5.apl_common"`. Must out-rank `comment`, which also starts
    // with `#`; the token precedence does that.
    include: ($) =>
      seq(
        field('directive', alias(token(prec(1, choice('#include', '#inline'))), $.directive)),
        $.string,
      ),

    comment: (_) => token(seq('#', /.*/)),

    // `define choice yesno_choice { "Yes" => "yes", … }` — the *second* word is
    // the type being reused, the *third* is the new name.
    define: ($) =>
      seq(
        'define',
        field('type', $.identifier),
        field('name', $.identifier),
        optional($.choice_list),
      ),

    section: ($) =>
      seq('section', field('name', $.identifier), field('body', $.block)),

    // `text { … }` or `text "de-DE" { … }` — the display labels, keyed by a
    // (possibly dotted) field path.
    text_block: ($) =>
      seq('text', optional(field('locale', $.string)), field('body', $.text_body)),

    text_body: ($) => seq('{', repeat($.text_entry), '}'),

    text_entry: ($) => seq(field('target', $.dotted_name), field('label', $.string)),

    table: ($) => seq('table', field('name', $.identifier), field('body', $.block)),

    block: ($) => seq('{', repeat($._block_item), '}'),

    _block_item: ($) =>
      choice($.field, $.message_field, $.table, $.optional_block, $.section),

    // `message "Welcome to the example iApp template."` — static display text.
    // Unlike every other field type, `message` is followed by a string literal
    // rather than a field name, so it cannot go through `field`.
    message_field: ($) => seq('message', field('text', $.string)),

    // `optional ( basic.protocol == "tcp" ) { … }` — APL's own tiny expression
    // language, in parentheses. Not Tcl, and not the `[ … ]` form.
    optional_block: ($) =>
      seq(
        'optional',
        '(',
        field('condition', $._expression),
        ')',
        field('body', $.block),
      ),

    _expression: ($) =>
      choice(
        $.binary_expression,
        $.dotted_name,
        $.string,
        $.number,
        $.variable_substitution,
        $.tcl_expression,
      ),

    binary_expression: ($) =>
      prec.left(
        seq(
          field('left', $._expression),
          field('operator', choice('==', '!=', '&&', '||', '<=', '>=', '<', '>')),
          field('right', $._expression),
        ),
      ),

    // `<type> <name> <attribute>* <choice_list>?`
    field: ($) =>
      seq(
        field('type', $.identifier),
        field('name', $.identifier),
        repeat($.attribute),
        optional($.choice_list),
      ),

    attribute: ($) =>
      choice(
        seq('default', field('value', choice($.string, $.number))),
        seq('display', field('value', $.string)),
        seq('validator', field('value', $.string)),
        'required',
      ),

    choice_list: ($) => seq('{', optional(commaSep($.choice_mapping)), optional(','), '}'),

    choice_mapping: ($) =>
      seq(field('label', $.string), '=>', field('value', $.string)),

    // A bracket expression *is* Tcl; `injections.scm` re-parses its content.
    tcl_expression: ($) => seq('[', repeat($._tcl_token), ']'),

    _tcl_token: ($) => choice($.tcl_expression, /[^\[\]]+/),

    dotted_name: ($) => seq($.identifier, repeat(seq('.', $.identifier))),

    // APL supports Tcl-style variable substitution: `$name`, `$ns::name`,
    // `$arr(idx)`, `${braced}`.
    variable_substitution: (_) =>
      token(seq('$', choice(/(?:[a-zA-Z0-9_]|::)+(?:\([^)]*\))?/, /\{[^}]*\}/))),

    identifier: (_) => /[A-Za-z_][A-Za-z0-9_-]*/,

    string: ($) =>
      seq(
        '"',
        repeat(choice($.escape_sequence, $.variable_substitution, /[^"\\$]+/)),
        '"',
      ),

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

    number: (_) => /[+-]?(?:0[xX][0-9a-fA-F]+|(?:[0-9]*\.)?[0-9]+f?)/,
  },
});

/** `a, b, c` — at least one item. */
function commaSep(rule) {
  return seq(rule, repeat(seq(',', rule)));
}

/** Every field type the grammar knows by name, for `highlights.scm`. */
module.exports.BUILTIN_FIELD_TYPES = BUILTIN_FIELD_TYPES;
