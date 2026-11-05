# BIG-IP Unicode logging: unicode dynamic

## Question

When UTF-8 byte sequences for accented letters, symbols and emoji are constructed with binary format H*, how do BIG-IP Tcl string length, binary scan and the log display treat those bytes? This constrains Unicode and bytearray modeling in the LSP, compiler and VM.

## Assumptions

This is the HTTP_REQUEST event body from the installed rule. Each line below is the observed log suffix after the unit and request fields; identical suffixes are grouped and their counts shown. Logged display bytes and the script's binary-scan hex are distinct observations. The results apply only to these inputs and this context.

## Script

```tcl-visible

    set unit [TMM::cmp_unit]
    set request [HTTP::header value X-R2286-Request]
    foreach item [list {precomposed c3a9} {decomposed 65cc81} {bmp_copyright c2a9} {bmp_snowman e29883} {bmp_heart e29da4} {emoji_grinning f09f9880} {emoji_text_vs e29da4efb88f} {emoji_skin_tone f09f918df09f8fbd} {emoji_zwj f09f91a9f09f8fbde2808df09f92bb} {emoji_family f09f91a8e2808df09f91a9e2808df09f91a7e2808df09f91a6} {emoji_flag f09f87b3f09f87bf} {emoji_rainbow_flag f09f8fb3efb88fe2808df09f8c88} {emoji_keycap 31efb88fe283a3}] {
        set label [lindex $item 0]
        set source_hex [lindex $item 1]
        set value [binary format H* $source_hex]
        binary scan $value H* value_hex
        log local0. "R2286|r2286i|unicode_dynamic|unit=$unit|request=$request|label=$label|source_hex=$source_hex|value=$value|value_hex=$value_hex|chars=[string length $value]"
    }
    HTTP::header insert X-R2286-Scope "unicode_dynamic:$unit"
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
label=bmp_copyright|source_hex=c2a9|value=Â©|value_hex=c2a9|chars=2
label=bmp_heart|source_hex=e29da4|value=â⟦U+009D⟧¤|value_hex=e29da4|chars=3
label=bmp_snowman|source_hex=e29883|value=â⟦U+0098⟧⟦U+0083⟧|value_hex=e29883|chars=3
label=decomposed|source_hex=65cc81|value=eÌ⟦U+0081⟧|value_hex=65cc81|chars=3
label=emoji_family|source_hex=f09f91a8e2808df09f91a9e2808df09f91a7e2808df09f91a6|value=ð⟦U+009F⟧⟦U+0091⟧¨â⟦U+0080⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+0091⟧©â⟦U+0080⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+0091⟧§â⟦U+0080⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+0091⟧¦|value_hex=f09f91a8e2808df09f91a9e2808df09f91a7e2808df09f91a6|chars=25
label=emoji_flag|source_hex=f09f87b3f09f87bf|value=ð⟦U+009F⟧⟦U+0087⟧³ð⟦U+009F⟧⟦U+0087⟧¿|value_hex=f09f87b3f09f87bf|chars=8
label=emoji_grinning|source_hex=f09f9880|value=ð⟦U+009F⟧⟦U+0098⟧⟦U+0080⟧|value_hex=f09f9880|chars=4
label=emoji_keycap|source_hex=31efb88fe283a3|value=1ï¸⟦U+008F⟧â⟦U+0083⟧£|value_hex=31efb88fe283a3|chars=7
label=emoji_rainbow_flag|source_hex=f09f8fb3efb88fe2808df09f8c88|value=ð⟦U+009F⟧⟦U+008F⟧³ï¸⟦U+008F⟧â⟦U+0080⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+008C⟧⟦U+0088⟧|value_hex=f09f8fb3efb88fe2808df09f8c88|chars=14
label=emoji_skin_tone|source_hex=f09f918df09f8fbd|value=ð⟦U+009F⟧⟦U+0091⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+008F⟧½|value_hex=f09f918df09f8fbd|chars=8
label=emoji_text_vs|source_hex=e29da4efb88f|value=â⟦U+009D⟧¤ï¸⟦U+008F⟧|value_hex=e29da4efb88f|chars=6
label=emoji_zwj|source_hex=f09f91a9f09f8fbde2808df09f92bb|value=ð⟦U+009F⟧⟦U+0091⟧©ð⟦U+009F⟧⟦U+008F⟧½â⟦U+0080⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+0092⟧»|value_hex=f09f91a9f09f8fbde2808df09f92bb|chars=15
label=precomposed|source_hex=c3a9|value=Ã©|value_hex=c3a9|chars=2
```

Each displayed suffix appeared in 64 log lines for this unit. The line breaks separate captured log suffixes.

### TMM unit 3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded.
- Recorded input channel: HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
label=bmp_copyright|source_hex=c2a9|value=Â©|value_hex=c2a9|chars=2
label=bmp_heart|source_hex=e29da4|value=â⟦U+009D⟧¤|value_hex=e29da4|chars=3
label=bmp_snowman|source_hex=e29883|value=â⟦U+0098⟧⟦U+0083⟧|value_hex=e29883|chars=3
label=decomposed|source_hex=65cc81|value=eÌ⟦U+0081⟧|value_hex=65cc81|chars=3
label=emoji_family|source_hex=f09f91a8e2808df09f91a9e2808df09f91a7e2808df09f91a6|value=ð⟦U+009F⟧⟦U+0091⟧¨â⟦U+0080⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+0091⟧©â⟦U+0080⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+0091⟧§â⟦U+0080⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+0091⟧¦|value_hex=f09f91a8e2808df09f91a9e2808df09f91a7e2808df09f91a6|chars=25
label=emoji_flag|source_hex=f09f87b3f09f87bf|value=ð⟦U+009F⟧⟦U+0087⟧³ð⟦U+009F⟧⟦U+0087⟧¿|value_hex=f09f87b3f09f87bf|chars=8
label=emoji_grinning|source_hex=f09f9880|value=ð⟦U+009F⟧⟦U+0098⟧⟦U+0080⟧|value_hex=f09f9880|chars=4
label=emoji_keycap|source_hex=31efb88fe283a3|value=1ï¸⟦U+008F⟧â⟦U+0083⟧£|value_hex=31efb88fe283a3|chars=7
label=emoji_rainbow_flag|source_hex=f09f8fb3efb88fe2808df09f8c88|value=ð⟦U+009F⟧⟦U+008F⟧³ï¸⟦U+008F⟧â⟦U+0080⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+008C⟧⟦U+0088⟧|value_hex=f09f8fb3efb88fe2808df09f8c88|chars=14
label=emoji_skin_tone|source_hex=f09f918df09f8fbd|value=ð⟦U+009F⟧⟦U+0091⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+008F⟧½|value_hex=f09f918df09f8fbd|chars=8
label=emoji_text_vs|source_hex=e29da4efb88f|value=â⟦U+009D⟧¤ï¸⟦U+008F⟧|value_hex=e29da4efb88f|chars=6
label=emoji_zwj|source_hex=f09f91a9f09f8fbde2808df09f92bb|value=ð⟦U+009F⟧⟦U+0091⟧©ð⟦U+009F⟧⟦U+008F⟧½â⟦U+0080⟧⟦U+008D⟧ð⟦U+009F⟧⟦U+0092⟧»|value_hex=f09f91a9f09f8fbde2808df09f92bb|chars=15
label=precomposed|source_hex=c3a9|value=Ã©|value_hex=c3a9|chars=2
```

Each displayed suffix appeared in 64 log lines for this unit. The line breaks separate captured log suffixes.
