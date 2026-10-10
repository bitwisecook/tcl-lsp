# naming.regex.jim-engine-program-range-matrix

Kind: `native-observation`

## Problem statement

The low-level Jim engine compiles a fixed pattern matrix into integer programs and reports byte match ranges. A script-level regexp result cannot identify its exact program layout, compile error taxonomy or flags, and neither can a C RegExp cache.

## Question

Which compile outcomes, integer programs and match ranges are retained for the thirty original Jim engine patterns under five flag vectors?

## Conclusion

The retained aggregate contains 150 pattern/flag blocks: 120 successful compile blocks and thirty compile-error blocks. Each successful block preserves its integer program and eight subject match/range rows. FF and modified-zero spellings remain distinct original byte patterns, while astral input is represented by its engine unit. These recorded blocks establish no missing subprocess status, general pattern equivalence, native object ownership or Rust parity.

## Scope

Direct jim_regcomp/jim_regexec C API probe, thirty fixed NUL-terminated pattern strings and eight fixed subjects, flags 0/2/4/8/32. Exact source/library/header/build identities are retained. The aggregate rows do not retain independent process exits for all 150 blocks, a runtime patch query or UTF configuration query; those properties remain unmeasured. C Tcl and BIG-IP were not tested.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### jim

Status: `observed`. Version: not queried; original source/library identities retained. Build: Original linked library SHA256 75c74eef0c9d1ff2ff8972e6339678564a3439d6332f66032eb4db79004937ff; probe executable SHA256 9b704d15aa2e0e146cb9dc811971944ce725cb415f1ecf930e8fa0de5dec50c1. Channel: Direct jim_regcomp/jim_regexec calls with original native C string pattern/subject arrays.. Dialect: Jim regexp engine.

The retained programs.txt contains thirty pattern indices under each of flags 0, 2, 4, 8 and 32, for 150 blocks. Compile codes: 120 code 0; five each of 8, 19, 13, 16, 18 and 10. Successful blocks include integer programs and eight subject rows. Build exit 0 is recorded, but per-block process status and runtime patch/UTF configuration queries are not retained.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

## Exact evidence

- `original-build` (provider): [rust/tcl-regex/tests/data/native_jim_regexp/manifest.json](../../../../rust/tcl-regex/tests/data/native_jim_regexp/manifest.json). SHA-256 `2a623a8e4d6ba5827cc0f8d9c1347406b01895a9e420eec7d06f6d31ca29ccbf`. Original exact probe build and linked Jim source/library/header identities.
- `original-probe` (input): [rust/tcl-regex/tests/data/native_jim_regexp/probe.c](../../../../rust/tcl-regex/tests/data/native_jim_regexp/probe.c). SHA-256 `c2eb3a6c97d6b3d638ec84c908545d1538e081a081b598315342fed546ed396f`. Exact fixed patterns/subjects and direct low-level compile/execute reporting.
- `original-matrix` (observation): [rust/tcl-regex/tests/data/native_jim_regexp/programs.txt](../../../../rust/tcl-regex/tests/data/native_jim_regexp/programs.txt). SHA-256 `e1833e37c5b6930ab3f900c16d73943c6aa3ac697287e494b3e08a6a6def9645`. Complete retained per-pattern/flag program, compile code/message and range rows; no missing process statuses are manufactured.
- `exact-source-2` (source-anchor): [docs/design/analysis/name-resolution-proofs/regexp-retained-source-anchors.json](../../../../docs/design/analysis/name-resolution-proofs/regexp-retained-source-anchors.json). SHA-256 `f36a41d7658d9b17accf0b4bf93dc17a1c11008bd8d1261ff9c10ce83f58678c`. JSON pointer `/2/snippet`. Exact current source excerpt whose complete source SHA equals the original captured source; independent of interpreter output and build revision query.

## Source inspection

jim Jim0.84 captured source label; no captured patch query, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03 (current source inspection; full-file digest equals captured source)`, `jimregexp.c`, function `jim_regcomp`, lines 284–342. Full-source SHA-256 `97cff9c0d8f2b0b6e09c40b46e77d09b80e667ea50aa2382e5e9b74552692acf`; snippet SHA-256 `f4bb5293214ea7920646df0d88c596fc0cc1d8b776c705d729184dcbc66c8324`; retained evidence `exact-source-2`.

```text
int jim_regcomp(regex_t *preg, const char *exp, int cflags)
{
	int scan;
	int longest;
	unsigned len;
	int flags;

#ifdef DEBUG
	fprintf(stderr, "Compiling: '%s'\n", exp);
#endif
	memset(preg, 0, sizeof(*preg));

	if (exp == NULL)
		FAIL(preg, REG_ERR_NULL_ARGUMENT);

	if (cflags & REG_EXPANDED) {
		preg->exp = reg_expanded_new_pattern(exp);
		exp = preg->exp;
	}

	/* First pass: determine size, legality. */
	preg->cflags = cflags;
	preg->regparse = exp;

	/* Allocate space. */
	preg->proglen = (strlen(exp) + 1) * 5;
	preg->program = malloc(preg->proglen * sizeof(int));
	if (preg->program == NULL)
		FAIL(preg, REG_ERR_NOMEM);

	/* Note that since we store a magic value as the first item in the program,
	 * program offsets will never be 0
	 */
	regc(preg, REG_MAGIC);
	if (reg(preg, 0, &flags) == 0) {
		return preg->err;
	}

	/* Small enough for pointer-storage convention? */
	if (preg->re_nsub >= REG_MAX_PAREN)		/* Probably could be 65535L. */
		FAIL(preg,REG_ERR_TOO_BIG);

	/* Dig out information for optimizations. */
	preg->regstart = 0;	/* Worst-case defaults. */
	preg->reganch = 0;
	preg->regmust = 0;
	preg->regmlen = 0;
	scan = 1;			/* First BRANCH. */
	if (OP(preg, regnext(preg, scan)) == END) {		/* Only one top-level choice. */
		scan = OPERAND(scan);

		/* Starting-point info. */
		if (OP(preg, scan) == EXACTLY) {
			preg->regstart = preg->program[OPERAND(scan)];
		}
		else if (OP(preg, scan) == BOL)
			preg->reganch++;

		/*

```


## Consumer bindings

- [rust/tcl-regex/src/jim.rs](../../../../rust/tcl-regex/src/jim.rs), `Regex`: Independent low-level Rust program/range implementation; recorded native outputs do not execute its assertions.
- [rust/tcl-regex/src/jim.rs](../../../../rust/tcl-regex/src/jim.rs), `jim::tests::original_jim_programs_and_ranges_match_all_150_native_controls` (linked): Checks its scoped original input/object/result windows against retained native controls. Adapter withdrawal after an aborted native attempt is a separate Rust contract; no executed Rust result is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

No replay runner or new execution is asserted. Rebuild the retained probe only with independently verified matching source/header/library identities, run each selected case in a fresh process and preserve stdout, stderr and process status separately. Original abnormal exits are retained observations of incomplete attempts; they are not successful completions and are not requested to be rerun.
