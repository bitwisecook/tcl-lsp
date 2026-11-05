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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No execution of this precise question on this provider is attached. Other interpreter or source-inspection results do not supply it.

### jim

Status: `observed`. Channel: Direct jim_regcomp/jim_regexec calls with original native C string pattern/subject arrays. Version: jim original source/library identities retained.

The retained programs.txt contains thirty pattern indices under each of flags 0, 2, 4, 8 and 32, for 150 blocks. Compile codes: 120 code 0; five each of 8, 19, 13, 16, 18 and 10. Successful blocks include integer programs and eight subject rows. Build exit 0 is recorded, but per-block process status and runtime patch/UTF configuration queries are not retained.

## Source inspection

jim Jim0.84 captured source label; no captured patch query, revision `5bac7c99ad65864c87da513e22e2f01703fa4e03 (current source inspection; full-file digest equals captured source)`, `jimregexp.c`, function `jim_regcomp`, lines 284–342. Retained reference `exact-source-2`.

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

## Evidence

- [rust/tcl-regex/tests/data/native_jim_regexp/manifest.json](../../../../rust/tcl-regex/tests/data/native_jim_regexp/manifest.json)
- [rust/tcl-regex/tests/data/native_jim_regexp/probe.c](../../../../rust/tcl-regex/tests/data/native_jim_regexp/probe.c)
- [rust/tcl-regex/tests/data/native_jim_regexp/programs.txt](../../../../rust/tcl-regex/tests/data/native_jim_regexp/programs.txt)
