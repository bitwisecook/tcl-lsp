# BIG-IP Unicode logging: unicode literal emoji skin tone

## Question

When a literal emoji with a skin-tone modifier is accepted in an iRule, what bytes, displayed value and string length does BIG-IP Tcl report? This constrains multi-code-point literal and byte-length modeling in the LSP, compiler and VM.

## Assumptions

This is the HTTP_REQUEST event body from the installed rule. Each line below is the observed log suffix after the unit and request fields; identical suffixes are grouped and their counts shown. Logged display bytes and the script's binary-scan hex are distinct observations. The results apply only to these inputs and this context.

## Script

```tcl-visible

    set unit [TMM::cmp_unit]
    set request [HTTP::header value X-R2286-Request]
    set value "👍🏽"
    binary scan $value H* value_hex
    log local0. "R2286|r2286i|unicode_literal_emoji_skin_tone|unit=$unit|request=$request|value=$value|value_hex=$value_hex|chars=[string length $value]"
    HTTP::header insert X-R2286-Scope "unicode_literal_emoji_skin_tone:$unit"
```

## Results

### TMM unit 0, HTTP_REQUEST; TMM unit 1, HTTP_REQUEST; TMM unit 2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
value=👍🏽|value_hex=f09f918df09f8fbd|chars=8
```

Each displayed suffix appeared in 16 log lines for this unit. The line breaks separate captured log suffixes.

### TMM unit 3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
value=👍🏽|value_hex=f09f918df09f8fbd|chars=8
```

Each displayed suffix appeared in 16 log lines for this unit. The line breaks separate captured log suffixes.
