//! Tcl code minifier — Rust port of `core/minifier/minifier.py`
//! (basic tier).
//!
//! Pure function: source in, minified source out, preserving
//! semantic equivalence by:
//!
//! 1. Stripping all comments.
//! 2. Collapsing inter-command whitespace to `;`.
//! 3. Collapsing intra-command whitespace to single spaces.
//! 4. Recursively minifying braced body arguments (and `[…]`
//!    command substitutions).
//! 5. Preserving string literals verbatim, dropping redundant
//!    double quotes when safe.
//! 6. Compressing whitespace inside `expr` bodies.
//! 7. Replacing `${var}` with `$var` when safe.
//! 8. Minifying `switch` braced case-list bodies individually.
//!
//! Deferred to later sub-strips (the opt-in tiers Python gates
//! behind flags, plus two default-path optimisations): AST-level
//! expression shrinking (De Morgan / comparison inversion),
//! template/`subst` deduplication, ensemble-subcommand
//! abbreviation, local-name compaction, namespace / string-literal
//! aliasing, and the symbol map.
//!
//! Note: the expression tokeniser adds a catch-all so no character
//! is dropped — the Python reference's `_EXPR_TOKEN` regex silently
//! drops unmatched characters (e.g. commas in `atan2($a,$b)` and
//! braces in `$x ni {a b}`), corrupting those expressions; this
//! port preserves them.
//!
//! Like the `type_hierarchy` provider, this is a pure-CPU core
//! provider that lands ahead of its LSP surface: the
//! `workspace/executeCommand` wiring (mirroring
//! `lsp/commands.py::on_minify_document`) is a follow-up.

use tcl_compiler::expr_ast::render_expr;
use tcl_compiler::{parse_expr, BinOp, ExprNode, UnaryOp};
use tcl_lexer::{Lexer, SourceMap, Token, TokenType};
use tcl_registry::{ArgRole, CommandRegistry};

/// One argument accumulated while parsing a command.
struct Arg {
    tokens: Vec<Token>,
    is_braced: bool,
    is_quoted: bool,
}

/// Minify a Tcl source string for the given dialect.
#[must_use]
pub fn minify_tcl(source: &str, dialect: &str, registry: &CommandRegistry) -> String {
    minify_body(source, dialect, registry)
}

/// Minify a Tcl script body (top-level or inside braces).
fn minify_body(source: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let sm = SourceMap::new(source);
    let Ok(tokens) = Lexer::new(source).tokenise_all() else {
        return source.to_owned();
    };

    let commands = parse_commands(source, &tokens);
    if commands.is_empty() {
        return String::new();
    }

    // Render each command, abbreviating ensemble subcommands.
    let mut rendered: Vec<Vec<String>> = Vec::with_capacity(commands.len());
    for cmd_args in &commands {
        let mut arg_strs = render_command(&sm, cmd_args, dialect, registry);
        if arg_strs.len() >= 2 {
            arg_strs[1] = abbreviated_subcommand(&arg_strs[0], &arg_strs[1], dialect);
        }
        rendered.push(arg_strs);
    }

    // Template deduplication (subst aliasing) of repeated dynamic
    // quoted args.
    let (template_map, rendered) = dedup_templates(rendered);

    let is_irules = dialect == "f5-irules";
    let mut parts: Vec<String> = Vec::new();
    for (content, alias) in &template_map {
        parts.push(format!("set {alias} {{{content}}}"));
    }
    for arg_strs in &rendered {
        if is_irules && arg_strs.len() > 1 {
            // In iRules, `}{` is a valid word boundary — omit the
            // space between adjacent braced args to save bytes.
            let mut piece = arg_strs[0].clone();
            for w in arg_strs.windows(2) {
                let (prev, cur) = (&w[0], &w[1]);
                if prev.ends_with('}') && cur.starts_with('{') {
                    piece.push_str(cur);
                } else {
                    piece.push(' ');
                    piece.push_str(cur);
                }
            }
            parts.push(piece);
        } else {
            parts.push(arg_strs.join(" "));
        }
    }
    parts.join(";")
}

/// Lazy generator of short identifier names: `a`, `b`, …, `z`,
/// `aa`, `ab`, …  Mirrors `core/common/text_edits.py::name_generator`.
struct NameGenerator {
    indices: Vec<usize>,
}

impl NameGenerator {
    fn new() -> Self {
        Self { indices: vec![0] }
    }

    fn next_name(&mut self) -> String {
        let name: String = self
            .indices
            .iter()
            .map(|&i| (b'a' + u8::try_from(i).unwrap_or(0)) as char)
            .collect();
        self.advance();
        name
    }

    fn advance(&mut self) {
        let mut pos = self.indices.len();
        loop {
            if pos == 0 {
                // All positions wrapped — grow the length.
                self.indices = vec![0; self.indices.len() + 1];
                return;
            }
            pos -= 1;
            if self.indices[pos] + 1 < 26 {
                self.indices[pos] += 1;
                return;
            }
            self.indices[pos] = 0;
        }
    }
}

/// Dialects where ensemble commands are fixed (no user-added
/// subcommands), so prefix abbreviation is safe.
const FIXED_ENSEMBLE_DIALECTS: &[&str] = &["f5-irules", "f5-iapps", "f5-bigip"];

/// Return the abbreviated subcommand text when safe for `dialect`.
/// Mirrors `_abbreviated_subcommand`.
fn abbreviated_subcommand(command_name: &str, subcommand_name: &str, dialect: &str) -> String {
    if !FIXED_ENSEMBLE_DIALECTS.contains(&dialect) {
        return subcommand_name.to_owned();
    }
    subcommand_abbreviation(command_name, subcommand_name)
        .unwrap_or(subcommand_name)
        .to_owned()
}

/// Shortest unambiguous abbreviation for `sub` of ensemble
/// `command`, or `None`.  Mirrors `_SUBCMD_ABBREVIATIONS` (only the
/// entries strictly shorter than the full subcommand are kept).
fn subcommand_abbreviation(command: &str, sub: &str) -> Option<&'static str> {
    let table: &[(&str, &str)] = match command {
        "string" => &[
            ("bytelength", "b"),
            ("cat", "ca"),
            ("compare", "co"),
            ("equal", "e"),
            ("first", "f"),
            ("index", "in"),
            ("last", "la"),
            ("length", "le"),
            ("match", "mat"),
            ("range", "ra"),
            ("repeat", "repe"),
            ("replace", "repl"),
            ("reverse", "rev"),
            ("tolower", "tol"),
            ("totitle", "tot"),
            ("toupper", "tou"),
            ("trimleft", "triml"),
            ("trimright", "trimr"),
            ("wordend", "worde"),
            ("wordstart", "words"),
        ],
        "info" => &[
            ("args", "a"),
            ("body", "b"),
            ("cmdcount", "cm"),
            ("commands", "comm"),
            ("complete", "comp"),
            ("default", "d"),
            ("exists", "e"),
            ("frame", "fr"),
            ("functions", "fu"),
            ("globals", "g"),
            ("hostname", "h"),
            ("level", "le"),
            ("library", "li"),
            ("loaded", "loa"),
            ("locals", "loc"),
            ("nameofexecutable", "n"),
            ("patchlevel", "pa"),
            ("procs", "pr"),
            ("script", "sc"),
            ("sharedlibextension", "sh"),
            ("tclversion", "t"),
        ],
        "clock" => &[
            ("add", "a"),
            ("clicks", "c"),
            ("format", "f"),
            ("microseconds", "mic"),
            ("milliseconds", "mil"),
            ("scan", "sc"),
            ("seconds", "se"),
        ],
        _ => return None,
    };
    table
        .iter()
        .find(|(full, _)| *full == sub)
        .map(|(_, abbr)| *abbr)
}

/// Replace repeated dynamic quoted args with `[subst $alias]` and a
/// shared `set alias {content}` preamble.  Mirrors
/// `_dedup_templates`; returns the ordered `(content, alias)`
/// preamble pairs plus the rewritten commands.
fn dedup_templates(rendered: Vec<Vec<String>>) -> (Vec<(String, String)>, Vec<Vec<String>>) {
    // content -> use sites, preserving first-seen order.
    let mut order: Vec<String> = Vec::new();
    let mut uses: std::collections::HashMap<String, Vec<(usize, usize)>> =
        std::collections::HashMap::new();
    for (ci, args) in rendered.iter().enumerate() {
        for (ai, s) in args.iter().enumerate() {
            if !(s.starts_with('"') && s.ends_with('"') && s.len() >= 2) {
                continue;
            }
            let content = &s[1..s.len() - 1];
            if !content.contains('$') && !content.contains('[') {
                continue;
            }
            if content.len() < 10 {
                continue;
            }
            if content.matches('{').count() != content.matches('}').count() {
                continue;
            }
            let key = content.to_owned();
            uses.entry(key.clone()).or_insert_with(|| {
                order.push(key.clone());
                Vec::new()
            });
            uses.get_mut(&key).expect("inserted").push((ci, ai));
        }
    }
    if order.is_empty() {
        return (Vec::new(), rendered);
    }

    // Names already referenced as $var, so aliases don't shadow them.
    let mut used_names = collect_used_var_names(&rendered);

    // Process candidates by descending count * content-length.
    let mut candidates = order.clone();
    candidates.sort_by(|a, b| {
        let ka = uses[a].len() * a.len();
        let kb = uses[b].len() * b.len();
        kb.cmp(&ka).then_with(|| {
            // Stable on first-seen order for ties.
            order
                .iter()
                .position(|x| x == a)
                .cmp(&order.iter().position(|x| x == b))
        })
    });

    let mut gen = NameGenerator::new();
    let mut template_map: Vec<(String, String)> = Vec::new();
    for content in &candidates {
        let count = uses[content].len();
        if count < 2 {
            continue;
        }
        let mut alias = gen.next_name();
        while used_names.contains(&alias) {
            alias = gen.next_name();
        }
        let original_cost = count * (content.len() + 2);
        let preamble_cost = 4 + alias.len() + 1 + 1 + content.len() + 1 + 1;
        let subst_ref = format!("[subst ${alias}]");
        let aliased_cost = preamble_cost + count * subst_ref.len();
        if aliased_cost >= original_cost {
            continue;
        }
        if content.contains(&format!("${alias}")) || content.contains(&format!("${{{alias}}}")) {
            continue;
        }
        template_map.push((content.clone(), alias.clone()));
        used_names.insert(alias);
    }
    if template_map.is_empty() {
        return (Vec::new(), rendered);
    }

    // Apply replacements.
    let mut result = rendered;
    for (content, alias) in &template_map {
        let subst_ref = format!("[subst ${alias}]");
        for &(ci, ai) in &uses[content] {
            result[ci][ai].clone_from(&subst_ref);
        }
    }
    (template_map, result)
}

/// Names referenced as `$var` / `${var}` anywhere in the rendered
/// commands.  Mirrors the `_used_var_re` scan in `_dedup_templates`.
fn collect_used_var_names(rendered: &[Vec<String>]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for args in rendered {
        for s in args {
            let bytes = s.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'$' {
                    let mut j = i + 1;
                    if j < bytes.len() && bytes[j] == b'{' {
                        j += 1;
                    }
                    let start = j;
                    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_')
                    {
                        j += 1;
                    }
                    if j > start {
                        out.insert(s[start..j].to_owned());
                    }
                    i = j;
                } else {
                    i += 1;
                }
            }
        }
    }
    out
}

/// Group a token stream into commands (lists of arguments),
/// dropping comments and whitespace.
fn parse_commands(source: &str, tokens: &[Token]) -> Vec<Vec<Arg>> {
    let mut commands: Vec<Vec<Arg>> = Vec::new();
    let mut current: Vec<Arg> = Vec::new();
    let mut prev_type = TokenType::Eol;

    for &tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Comment => continue,
            TokenType::Sep => {
                prev_type = TokenType::Sep;
                continue;
            }
            TokenType::Eol => {
                if !current.is_empty() {
                    commands.push(std::mem::take(&mut current));
                }
                prev_type = TokenType::Eol;
                continue;
            }
            _ => {}
        }

        let is_start = matches!(prev_type, TokenType::Sep | TokenType::Eol);
        let detected_quoted =
            is_start && source.as_bytes().get(tok.span.start() as usize) == Some(&b'"');

        if is_start || current.is_empty() {
            current.push(Arg {
                tokens: vec![tok],
                is_braced: tok.kind == TokenType::Str,
                is_quoted: detected_quoted,
            });
        } else {
            current.last_mut().expect("non-empty").tokens.push(tok);
        }
        prev_type = tok.kind;
    }
    if !current.is_empty() {
        commands.push(current);
    }
    commands
}

/// Render one command's arguments to their minified string forms.
fn render_command(
    sm: &SourceMap,
    cmd_args: &[Arg],
    dialect: &str,
    registry: &CommandRegistry,
) -> Vec<String> {
    let cmd_name = cmd_args
        .first()
        .map(|a| token_text(sm, a))
        .unwrap_or_default();
    let post: Vec<String> = cmd_args.iter().skip(1).map(|a| token_text(sm, a)).collect();
    let post_refs: Vec<&str> = post.iter().map(String::as_str).collect();

    let body_indices = role_indices(registry, &cmd_name, &post_refs, ArgRole::Body);
    let expr_indices = role_indices(registry, &cmd_name, &post_refs, ArgRole::Expr);
    let is_case_list = cmd_name == "switch" && is_switch_case_list_form(&post_refs);

    let mut out: Vec<String> = Vec::with_capacity(cmd_args.len());
    for (i, arg) in cmd_args.iter().enumerate() {
        let single_braced = arg.is_braced && arg.tokens.len() == 1;
        if body_indices.contains(&i) && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            let minified = if is_case_list {
                minify_switch_case_list(inner, dialect, registry)
            } else {
                minify_body(inner, dialect, registry)
            };
            out.push(format!("{{{minified}}}"));
        } else if expr_indices.contains(&i) && single_braced {
            let inner = sm.token_text(arg.tokens[0]);
            out.push(format!("{{{}}}", compress_expr(inner, dialect, registry)));
        } else {
            out.push(reconstruct_arg(sm, arg, dialect, registry));
        }
    }
    out
}

/// Registry role indices, offset by 1 for the command-name slot.
fn role_indices(
    registry: &CommandRegistry,
    name: &str,
    post_args: &[&str],
    role: ArgRole,
) -> Vec<usize> {
    if name.is_empty() {
        return Vec::new();
    }
    registry
        .arg_indices_for_role(name, post_args, role)
        .into_iter()
        .map(|i| i + 1)
        .collect()
}

/// Text of an argument's first token (Python's `_token_text`).
fn token_text(sm: &SourceMap, arg: &Arg) -> String {
    arg.tokens
        .first()
        .map(|&t| sm.token_text(t).to_owned())
        .unwrap_or_default()
}

/// First character a token will render as.  Mirrors
/// `_first_rendered_char`.
fn first_rendered_char(sm: &SourceMap, tok: Token) -> Option<char> {
    match tok.kind {
        TokenType::Str | TokenType::Expand => Some('{'),
        TokenType::Cmd => Some('['),
        TokenType::Var => Some('$'),
        _ => sm.token_text(tok).chars().next(),
    }
}

/// Rebuild source text from a single token, re-adding delimiters
/// and recursively minifying `[…]` substitutions.  Mirrors
/// `_reconstruct_raw`.
fn reconstruct_raw(
    sm: &SourceMap,
    tok: Token,
    next_tok: Option<Token>,
    dialect: &str,
    registry: &CommandRegistry,
) -> String {
    match tok.kind {
        TokenType::Str => format!("{{{}}}", sm.token_text(tok)),
        TokenType::Cmd => format!("[{}]", minify_body(sm.token_text(tok), dialect, registry)),
        TokenType::Var => {
            // Inside a quoted string, keep `${var}` when the next
            // token would otherwise extend the variable name.
            if let Some(next) = next_tok {
                if let Some(c) = first_rendered_char(sm, next) {
                    if c.is_alphanumeric() || c == '_' {
                        return format!("${{{}}}", sm.token_text(tok));
                    }
                }
            }
            format!("${}", sm.token_text(tok))
        }
        TokenType::Expand => "{*}".to_owned(),
        _ => sm.token_text(tok).to_owned(),
    }
}

/// Characters that would change semantics if they appear unquoted.
const NEEDS_QUOTING: &[char] = &[' ', '\t', '\n', '\r', '\u{0b}', '\u{0c}', ';', '"', '\0'];

/// Whether a quoted argument can safely drop its double quotes.
/// Mirrors `_can_strip_quotes`.
fn can_strip_quotes(raw: &str) -> bool {
    if raw.is_empty() {
        return false;
    }
    let first = raw.chars().next().unwrap();
    if matches!(first, '"' | '{' | '#') {
        return false;
    }
    if raw == "{*}" {
        return false;
    }
    if raw.chars().any(|c| NEEDS_QUOTING.contains(&c)) {
        return false;
    }
    // Any `{` / `}` outside `${var}` references blocks stripping.
    let stripped = strip_braced_var_refs(raw);
    !(stripped.contains('{') || stripped.contains('}'))
}

/// Remove `${…}` references from `raw` so the residual brace check
/// in [`can_strip_quotes`] only sees bare braces.
fn strip_braced_var_refs(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let n = bytes.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    while i < n {
        if bytes[i] == b'$' && i + 1 < n && bytes[i + 1] == b'{' {
            if let Some(close) = raw[i + 2..].find('}') {
                i = i + 2 + close + 1;
                continue;
            }
        }
        let ch_len = utf8_len(bytes[i]);
        out.push_str(&raw[i..i + ch_len]);
        i += ch_len;
    }
    out
}

/// Byte length of the UTF-8 char whose lead byte is `b`.
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

/// Rebuild the source text of an argument from its tokens.
/// Mirrors `_reconstruct_arg`.
fn reconstruct_arg(sm: &SourceMap, arg: &Arg, dialect: &str, registry: &CommandRegistry) -> String {
    let mut raw = String::new();
    for (idx, &tok) in arg.tokens.iter().enumerate() {
        let next = if arg.is_quoted {
            arg.tokens.get(idx + 1).copied()
        } else {
            None
        };
        raw.push_str(&reconstruct_raw(sm, tok, next, dialect, registry));
    }
    if arg.is_quoted && !can_strip_quotes(&raw) {
        format!("\"{raw}\"")
    } else {
        raw
    }
}

// ---------------------------------------------------------------------------
// switch case-list handling
// ---------------------------------------------------------------------------

/// Whether the post-name args use the braced case-list form (a
/// single trailing word after any leading options).  Mirrors
/// `is_switch_case_list_form`.
fn is_switch_case_list_form(args: &[&str]) -> bool {
    let i = skip_switch_options(args);
    i < args.len() && i == args.len() - 1
}

/// Skip leading `switch` option words and the match-value arg,
/// returning the index of the first case-list element.  Mirrors
/// `_skip_switch_options` (options, then one value arg).
fn skip_switch_options(args: &[&str]) -> usize {
    let mut i = 0;
    while i < args.len() {
        let a = args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            // `-matchvar` / `-indexvar` consume a following value.
            if matches!(a, "-matchvar" | "-indexvar") {
                i += 1;
            }
            i += 1;
        } else {
            break;
        }
    }
    // Skip the match-value argument itself.
    if i < args.len() {
        i += 1;
    }
    i
}

/// Minify the content of a `switch` braced case list, recursively
/// minifying each braced body.  Mirrors `_minify_switch_case_list`.
fn minify_switch_case_list(source: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let sm = SourceMap::new(source);
    let Ok(tokens) = Lexer::new(source).tokenise_all() else {
        return source.to_owned();
    };
    // Segment into words (pattern / body), grouping multi-token words.
    let mut words: Vec<(String, bool, Token)> = Vec::new(); // (raw, is_braced, first_tok)
    let mut prev_type = TokenType::Eol;
    for tok in tokens {
        match tok.kind {
            TokenType::Eof => break,
            TokenType::Sep | TokenType::Eol | TokenType::Comment => {
                prev_type = tok.kind;
                continue;
            }
            _ => {}
        }
        let raw = reconstruct_raw(&sm, tok, None, dialect, registry);
        if matches!(
            prev_type,
            TokenType::Sep | TokenType::Eol | TokenType::Comment
        ) || words.is_empty()
        {
            words.push((raw, tok.kind == TokenType::Str, tok));
        } else {
            words.last_mut().expect("non-empty").0.push_str(&raw);
        }
        prev_type = tok.kind;
    }

    let mut parts: Vec<String> = Vec::new();
    let mut idx = 0;
    while idx + 1 < words.len() {
        let pattern = &words[idx].0;
        let (body_raw, body_braced, body_tok) = &words[idx + 1];
        let body_inner = sm.token_text(*body_tok);
        if body_inner == "-" && *body_raw == "-" {
            parts.push(format!("{pattern} -"));
        } else if *body_braced {
            let minified = minify_body(body_inner, dialect, registry);
            parts.push(format!("{pattern} {{{minified}}}"));
        } else {
            parts.push(format!("{pattern} {body_raw}"));
        }
        idx += 2;
    }
    parts.join(" ")
}

// ---------------------------------------------------------------------------
// expr whitespace compression
// ---------------------------------------------------------------------------

/// One token of an `expr` body for whitespace compression.
enum ExprTok {
    /// A `[…]` command substitution (already minified).
    Cmd(String),
    /// Any other token (string, var, word, operator, punctuation).
    Other(String),
    /// A run of whitespace.
    Space,
}

/// Remove unnecessary whitespace inside an `expr` body, keeping
/// spaces only around word-operators and between adjacent word
/// tokens.  Mirrors `_strip_expr_whitespace` (no AST shrinking).
fn strip_expr_whitespace(text: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let toks = tokenise_expr(text, dialect, registry);
    let rendered: Vec<String> = toks
        .iter()
        .filter_map(|t| match t {
            ExprTok::Space => None,
            ExprTok::Cmd(s) | ExprTok::Other(s) => Some(s.clone()),
        })
        .collect();
    if rendered.is_empty() {
        return text.to_owned();
    }
    let mut out = String::new();
    out.push_str(&rendered[0]);
    for w in rendered.windows(2) {
        let (prev, cur) = (&w[0], &w[1]);
        if is_word_op(prev) || is_word_op(cur) || (is_word_token(prev) && is_word_token(cur)) {
            out.push(' ');
        }
        out.push_str(cur);
    }
    out
}

/// Compress and shrink an `expr` body: strip whitespace, then try
/// AST transforms (De Morgan / comparison inversion / double
/// negation) and keep whichever is shorter.  Mirrors
/// `_compress_expr`.
fn compress_expr(text: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let compressed = strip_expr_whitespace(text, dialect, registry);
    let shrunk = shrink_expr_ast(&compressed, dialect, registry);
    if shrunk.len() < compressed.len() {
        shrunk
    } else {
        compressed
    }
}

/// AST-based expression shrinking.  Mirrors `_shrink_expr_ast`.
fn shrink_expr_ast(text: &str, dialect: &str, registry: &CommandRegistry) -> String {
    let node = parse_expr(text, Some(dialect));
    if matches!(node, ExprNode::Raw { .. }) {
        return text.to_owned();
    }
    let shrunk = shrink_node(&node);
    if shrunk == node {
        return text.to_owned();
    }
    let rendered = render_expr(&shrunk);
    strip_expr_whitespace(&rendered, dialect, registry)
}

/// The logical complement of a comparison / membership operator,
/// or `None` when it has none.  Mirrors `_COMPARISON_INVERSION`.
fn comparison_inversion(op: BinOp) -> Option<BinOp> {
    Some(match op {
        BinOp::Eq => BinOp::Ne,
        BinOp::Ne => BinOp::Eq,
        BinOp::Lt => BinOp::Ge,
        BinOp::Ge => BinOp::Lt,
        BinOp::Gt => BinOp::Le,
        BinOp::Le => BinOp::Gt,
        BinOp::StrEq => BinOp::StrNe,
        BinOp::StrNe => BinOp::StrEq,
        BinOp::In => BinOp::Ni,
        BinOp::Ni => BinOp::In,
        BinOp::StrLt => BinOp::StrGe,
        BinOp::StrGe => BinOp::StrLt,
        BinOp::StrGt => BinOp::StrLe,
        BinOp::StrLe => BinOp::StrGt,
        _ => return None,
    })
}

/// Build a `!operand` node.
fn negate(operand: ExprNode) -> ExprNode {
    ExprNode::Unary {
        op: UnaryOp::Not,
        operand: Box::new(operand),
    }
}

/// Pick `candidate` over `original` when its rendering is shorter.
fn pick_shorter(candidate: ExprNode, original: &ExprNode) -> ExprNode {
    if render_expr(&candidate).len() < render_expr(original).len() {
        candidate
    } else {
        original.clone()
    }
}

/// Recursively try size-reducing transforms on an expression node.
/// Mirrors `_shrink_node`.
fn shrink_node(node: &ExprNode) -> ExprNode {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => shrink_not(node, operand),
        ExprNode::Binary { op, left, right }
            if matches!(op, BinOp::Or | BinOp::WordOr) && both_negations(left, right) =>
        {
            // De Morgan reverse: !a || !b → !(a && b) (if shorter).
            let (a, b) = (unwrap_not(left), unwrap_not(right));
            let dual = if *op == BinOp::Or {
                BinOp::And
            } else {
                BinOp::WordAnd
            };
            let combined = negate(ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(a)),
                right: Box::new(shrink_node(b)),
            });
            pick_shorter(combined, node)
        }
        ExprNode::Binary { op, left, right }
            if matches!(op, BinOp::And | BinOp::WordAnd) && both_negations(left, right) =>
        {
            // De Morgan reverse: !a && !b → !(a || b) (if shorter).
            let (a, b) = (unwrap_not(left), unwrap_not(right));
            let dual = if *op == BinOp::And {
                BinOp::Or
            } else {
                BinOp::WordOr
            };
            let combined = negate(ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(a)),
                right: Box::new(shrink_node(b)),
            });
            pick_shorter(combined, node)
        }
        ExprNode::Binary { op, left, right } => {
            let new_left = shrink_node(left);
            let new_right = shrink_node(right);
            ExprNode::Binary {
                op: *op,
                left: Box::new(new_left),
                right: Box::new(new_right),
            }
        }
        ExprNode::Unary { op, operand } => ExprNode::Unary {
            op: *op,
            operand: Box::new(shrink_node(operand)),
        },
        ExprNode::Ternary {
            condition,
            true_branch,
            false_branch,
        } => ExprNode::Ternary {
            condition: Box::new(shrink_node(condition)),
            true_branch: Box::new(shrink_node(true_branch)),
            false_branch: Box::new(shrink_node(false_branch)),
        },
        other => other.clone(),
    }
}

/// Whether both operands are `!`-negations.
fn both_negations(left: &ExprNode, right: &ExprNode) -> bool {
    matches!(
        left,
        ExprNode::Unary {
            op: UnaryOp::Not,
            ..
        }
    ) && matches!(
        right,
        ExprNode::Unary {
            op: UnaryOp::Not,
            ..
        }
    )
}

/// The operand of a `!`-negation (caller guarantees the shape).
fn unwrap_not(node: &ExprNode) -> &ExprNode {
    match node {
        ExprNode::Unary {
            op: UnaryOp::Not,
            operand,
        } => operand,
        _ => node,
    }
}

/// Handle the `!`-prefixed shrink cases (double negation,
/// comparison inversion, De Morgan forward), falling back to a
/// generic operand recurse.
fn shrink_not(node: &ExprNode, operand: &ExprNode) -> ExprNode {
    // Double negation: !!x → x.
    if let ExprNode::Unary {
        op: UnaryOp::Not,
        operand: inner,
    } = operand
    {
        return shrink_node(inner);
    }
    if let ExprNode::Binary { op, left, right } = operand {
        // Comparison inversion: !($a == $b) → $a != $b.
        if let Some(inv) = comparison_inversion(*op) {
            let inverted = ExprNode::Binary {
                op: inv,
                left: Box::new(shrink_node(left)),
                right: Box::new(shrink_node(right)),
            };
            return pick_shorter(inverted, node);
        }
        // De Morgan forward.
        if matches!(op, BinOp::And | BinOp::WordAnd | BinOp::Or | BinOp::WordOr) {
            let neg_l = negate(shrink_node(left));
            let neg_r = negate(shrink_node(right));
            let dual = match op {
                BinOp::And => BinOp::Or,
                BinOp::WordAnd => BinOp::WordOr,
                BinOp::Or => BinOp::And,
                _ => BinOp::WordAnd,
            };
            let demorgan = ExprNode::Binary {
                op: dual,
                left: Box::new(shrink_node(&neg_l)),
                right: Box::new(shrink_node(&neg_r)),
            };
            return pick_shorter(demorgan, node);
        }
    }
    // Generic recurse into the operand.
    negate(shrink_node(operand))
}

/// Tokenise an `expr` body, mirroring the `_EXPR_TOKEN` alternation
/// (with a catch-all so no character is dropped — safer than the
/// Python reference, which silently drops unmatched characters).
fn tokenise_expr(text: &str, dialect: &str, registry: &CommandRegistry) -> Vec<ExprTok> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            let start = i;
            while i < n && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let _ = start;
            out.push(ExprTok::Space);
        } else if c == b'"' {
            let start = i;
            i += 1;
            while i < n {
                if bytes[i] == b'\\' && i + 1 < n {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else if c == b'[' {
            let start = i;
            i += 1;
            let mut depth = 1;
            while i < n && depth > 0 {
                match bytes[i] {
                    b'[' => depth += 1,
                    b']' => depth -= 1,
                    _ => {}
                }
                i += 1;
            }
            let inner = &text[start + 1..i.saturating_sub(1).max(start + 1)];
            out.push(ExprTok::Cmd(format!(
                "[{}]",
                minify_body(inner, dialect, registry)
            )));
        } else if c == b'$' {
            let start = i;
            i += 1;
            while i < n
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':')
            {
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else if c.is_ascii_alphanumeric() || c == b'.' || c == b'_' {
            let start = i;
            while i < n
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'_')
            {
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else if is_expr_op_byte(c) {
            let start = i;
            while i < n && is_expr_op_byte(bytes[i]) {
                i += 1;
            }
            out.push(ExprTok::Other(text[start..i].to_owned()));
        } else {
            // Catch-all single char (`(`, `)`, `,`, etc.).
            let ch_len = utf8_len(c);
            out.push(ExprTok::Other(text[i..i + ch_len].to_owned()));
            i += ch_len;
        }
    }
    out
}

/// Whether `b` is a byte that forms a symbolic `expr` operator.
fn is_expr_op_byte(b: u8) -> bool {
    matches!(
        b,
        b'+' | b'-'
            | b'*'
            | b'/'
            | b'%'
            | b'<'
            | b'>'
            | b'='
            | b'!'
            | b'&'
            | b'|'
            | b'^'
            | b'?'
            | b':'
            | b'~'
    )
}

/// Whether `tok` is a Tcl expr word-operator needing surrounding
/// whitespace (`eq`, `ne`, `in`, `ni`).
fn is_word_op(tok: &str) -> bool {
    matches!(tok, "eq" | "ne" | "in" | "ni")
}

/// Whether `tok` is a "word" (identifier / number / variable /
/// string / command-substitution).  Mirrors `_is_word_token`.
fn is_word_token(tok: &str) -> bool {
    let Some(c) = tok.chars().next() else {
        return false;
    };
    c == '$' || c == '"' || c == '[' || c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn min(src: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl(src, "tcl8.6", &registry)
    }

    fn check(input: &str, expected: &str) {
        let got = min(input);
        assert_eq!(
            got, expected,
            "\ninput:    {input:?}\ngot:      {got:?}\nexpected: {expected:?}"
        );
    }

    #[test]
    fn strips_comments() {
        check("# a comment\nputs hi\n", "puts hi");
    }

    fn min_dialect(src: &str, dialect: &str) -> String {
        let registry = CommandRegistry::build_default();
        minify_tcl(src, dialect, &registry)
    }

    #[test]
    fn dedup_repeated_dynamic_templates() {
        check(
            "puts \"value is $longvariablename here\"\nputs \"value is $longvariablename here\"\n",
            "set a {value is $longvariablename here};puts [subst $a];puts [subst $a]",
        );
    }

    #[test]
    fn abbreviates_ensemble_subcommand_in_irules() {
        assert_eq!(
            min_dialect("string length $x\n", "f5-irules"),
            "string le $x"
        );
        assert_eq!(min_dialect("info exists $x\n", "f5-irules"), "info e $x");
    }

    #[test]
    fn no_subcommand_abbreviation_in_plain_tcl() {
        assert_eq!(
            min_dialect("string length $x\n", "tcl8.6"),
            "string length $x"
        );
    }

    #[test]
    fn expr_comparison_inversion() {
        check("if {!($a == $b)} {puts x}\n", "if {$a!=$b} {puts x}");
    }

    #[test]
    fn expr_de_morgan_forward() {
        check("if {!($a && $b)} {puts x}\n", "if {!$a||!$b} {puts x}");
    }

    #[test]
    fn expr_de_morgan_reverse() {
        check("if {!$a || !$b} {puts x}\n", "if {!$a||!$b} {puts x}");
    }

    #[test]
    fn expr_double_negation() {
        check("if {!!$x} {puts x}\n", "if {$x} {puts x}");
    }

    #[test]
    fn expr_no_change_when_already_minimal() {
        check("if {$a < $b} {puts x}\n", "if {$a<$b} {puts x}");
    }

    #[test]
    fn expr_shrink_nested_in_command_subst() {
        check(
            "set y [expr {!($a==1 && $b==2)}]\n",
            "set y [expr {$a!=1||$b!=2}]",
        );
    }

    #[test]
    fn collapses_commands_to_semicolons() {
        check("set x 1\nset y 2\n", "set x 1;set y 2");
    }

    #[test]
    fn collapses_intra_command_whitespace() {
        check("set    x     1\n", "set x 1");
    }

    #[test]
    fn recurses_into_proc_body() {
        check(
            "proc f {} {\n    # c\n    set x 1\n}\n",
            "proc f {} {set x 1}",
        );
    }

    #[test]
    fn recurses_into_command_substitution() {
        check("set y [ expr {1 + 2} ]\n", "set y [expr {1+2}]");
    }

    #[test]
    fn strips_redundant_quotes() {
        check("puts \"hello\"\n", "puts hello");
    }

    #[test]
    fn keeps_quotes_when_needed() {
        check("puts \"hello world\"\n", "puts \"hello world\"");
    }

    #[test]
    fn compresses_expr_whitespace() {
        check("if {$a == 1} {\n    puts hi\n}\n", "if {$a==1} {puts hi}");
    }

    #[test]
    fn keeps_word_operator_spacing() {
        check(
            "if {$a eq $b} {\n    puts hi\n}\n",
            "if {$a eq $b} {puts hi}",
        );
    }

    #[test]
    fn minifies_switch_case_bodies() {
        check(
            "switch $x {\n    a {\n        puts 1\n    }\n    b {\n        puts 2\n    }\n}\n",
            "switch $x {a {puts 1} b {puts 2}}",
        );
    }

    #[test]
    fn switch_fallthrough_preserved() {
        check(
            "switch $x {\n    a -\n    b {\n        puts 2\n    }\n}\n",
            "switch $x {a - b {puts 2}}",
        );
    }

    #[test]
    fn nested_body_recursion() {
        check(
            "proc f {} {\n    if {$x} {\n        set y 1\n    }\n}\n",
            "proc f {} {if {$x} {set y 1}}",
        );
    }

    #[test]
    fn empty_source_minifies_to_empty() {
        check("\n\n# only a comment\n", "");
    }
}
