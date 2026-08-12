# Port of rust/tcl-registry/src/commands/irules/http__header.rs
#
# Exercises: an iRules taint source that is also a subcommand-scoped output
# sink, layer/profile event requirements, a dialect-only command, and
# subcommand-level credential / sensitive-header facts.

speclib f5-irules 1.0 {

command HTTP::header {
    dialects f5-irules
    traits {PURE CSE_CANDIDATE DIAGRAM_ACTION}
    arity 1..

    option -noupdate -detail {Do not propagate the header mutation to subsequent BIG-IP filters.}

    # The command's result is client-controlled data.
    taint_source {TAINTED}

    # `HTTP::header insert|replace` with tainted data → header injection.
    taint_output_sink IRULE3002
    taint_output_sink_subcommands {insert replace}

    # Layer-based validity for the IRULE1001 check.  Unlisted booleans
    # (client_side, server_side, init_only, flow) stay false and `capability`
    # stays unset.
    event_requires {
        transport tcp
        profiles  {FASTHTTP HTTP}
        also_in   {MR_EGRESS MR_INGRESS SERVER_CONNECTED}
    }

    side_effect HttpHeader -reads -writes -side Both

    form Default {HTTP::header <subcommand> ?arg ...?}

    hover {
        summary  {Inspect or mutate HTTP headers in an iRule event.}
        synopsis {HTTP::header <subcommand> ?arg ...?}
        description {Use subcommands like `value`, `insert`, `replace`, and `remove`.}
        source   {https://clouddocs.f5.com/api/irules/HTTP__header.html}
    }

    subcommand at       { arity 1 ;    detail {Get header name by index.} ; synopsis {HTTP::header at <index>} }
    subcommand count    { arity 0..1 ; detail {Count headers.} ;           synopsis {HTTP::header count ?<name>?} }
    subcommand exists   { arity 1 ;    detail {Check if header exists.} ;  synopsis {HTTP::header exists <name>} }

    subcommand insert {
        arity 2..
        detail   {Insert header/value pair.}
        synopsis {HTTP::header insert <name> <value>}
        mutator
        # Carried verbatim from the shipped spec.
        credential_arg 2
        sensitive_headers {authorization proxy-authorization x-api-key x-auth-token x-secret}
    }

    subcommand insert_modssl_fields {
        arity 2..3
        detail   {Insert mod_ssl-compatible client IP/port headers.}
        synopsis {HTTP::header insert_modssl_fields <addr port | addr addr addr | port port port>}
        mutator
    }

    subcommand is_keepalive { arity 0 ; detail {Check if connection is keep-alive.} ; synopsis {HTTP::header is_keepalive} }
    subcommand is_redirect  { arity 0 ; detail {Check if response is a redirect.} ;   synopsis {HTTP::header is_redirect} }
    subcommand lws          { arity 0 ; detail {Returns 1 if a header with linear white space was encountered.} ; synopsis {HTTP::header lws} }
    subcommand names        { arity 0 ; detail {List all header names.} ;           synopsis {HTTP::header names} }

    subcommand remove {
        arity 1
        detail   {Remove named header.}
        synopsis {HTTP::header remove <name>}
        mutator
    }

    subcommand replace {
        arity 1..2
        detail   {Replace header value.}
        synopsis {HTTP::header replace <name> ?<string>?}
        mutator
        credential_arg 2
        sensitive_headers {authorization proxy-authorization x-api-key x-auth-token x-secret}
    }

    subcommand sanitize {
        arity 1..
        detail   {Remove headers not in the allow list.}
        synopsis {HTTP::header sanitize <name-list>}
        mutator
    }

    subcommand value  { arity 1 ; detail {Get first header value.} ;   synopsis {HTTP::header value <name>} }
    subcommand values { arity 1 ; detail {Get all values for header.} ; synopsis {HTTP::header values <name>} }
}

}
