
when HTTP_REQUEST {
    set unit [TMM::cmp_unit]
    set request [HTTP::header value X-R2286-Request]
    set value "❤"
    binary scan $value H* value_hex
    log local0. "R2286|r2286i|unicode_literal_bmp_heart|unit=$unit|request=$request|value=$value|value_hex=$value_hex|chars=[string length $value]"
    HTTP::header insert X-R2286-Scope "unicode_literal_bmp_heart:$unit"
}
when HTTP_RESPONSE {
    HTTP::header insert X-R2286-TMM [TMM::cmp_unit]
}
