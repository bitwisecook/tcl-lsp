# _event_data.tcl -- AUTO-GENERATED from Python event registry
#
# DO NOT EDIT.  Regenerate with:
#   python -m core.irule_test.codegen_event_data
#
# Source: core/commands/registry/event_flow_chains.py
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

namespace eval ::orch {

    # Master event ordering
    #
    # Each entry: {event_name profile_gates}
    # Profile gates: empty = always relevant; non-empty = requires
    # at least one of these profiles to be active.

    variable MASTER_ORDER {
        {RULE_INIT                                {}}
        {FLOW_INIT                                {FLOW}}
        {CLIENT_ACCEPTED                          {}}
        {CLIENT_DATA                              {}}
        {CLIENTSSL_CLIENTHELLO                    {CLIENTSSL}}
        {CLIENTSSL_SERVERHELLO_SEND               {CLIENTSSL}}
        {CLIENTSSL_CLIENTCERT                     {CLIENTSSL}}
        {CLIENTSSL_HANDSHAKE                      {CLIENTSSL}}
        {CLIENTSSL_DATA                           {CLIENTSSL}}
        {CLIENTSSL_PASSTHROUGH                    {CLIENTSSL}}
        {HTTP_REQUEST                             {FASTHTTP HTTP}}
        {HTTP_REQUEST_DATA                        {HTTP}}
        {HTTP_PROXY_REQUEST                       {HTTP}}
        {AUTH_RESULT                              {AUTH}}
        {AUTH_SUCCESS                             {AUTH}}
        {AUTH_FAILURE                             {AUTH}}
        {AUTH_ERROR                               {AUTH}}
        {AUTH_WANTCREDENTIAL                      {AUTH}}
        {ACCESS_SESSION_STARTED                   {ACCESS}}
        {ACCESS_POLICY_AGENT_EVENT                {ACCESS}}
        {ACCESS_POLICY_COMPLETED                  {ACCESS}}
        {CLASSIFICATION_DETECTED                  {CLASSIFICATION}}
        {CATEGORY_MATCHED                         {CATEGORY}}
        {HTTP_CLASS_SELECTED                      {HTTP}}
        {HTTP_CLASS_FAILED                        {HTTP}}
        {CACHE_REQUEST                            {CACHE WEBACCELERATION}}
        {CACHE_RESPONSE                           {CACHE WEBACCELERATION}}
        {IN_DOSL7_ATTACK                          {DOSL7}}
        {ASM_REQUEST_DONE                         {ASM}}
        {ASM_REQUEST_VIOLATION                    {ASM}}
        {ASM_REQUEST_BLOCKING                     {ASM}}
        {DNS_REQUEST                              {DNS}}
        {SIP_REQUEST                              {SIP}}
        {PERSIST_DOWN                             {}}
        {LB_SELECTED                              {}}
        {LB_FAILED                                {}}
        {LB_QUEUED                                {}}
        {SA_PICKED                                {}}
        {SERVER_INIT                              {}}
        {ACCESS_ACL_ALLOWED                       {ACCESS}}
        {ACCESS_ACL_DENIED                        {ACCESS}}
        {ACCESS_PER_REQUEST_AGENT_EVENT           {ACCESS}}
        {REWRITE_REQUEST_DONE                     {REWRITE}}
        {SERVER_CONNECTED                         {}}
        {SERVERSSL_CLIENTHELLO_SEND               {SERVERSSL}}
        {SERVERSSL_SERVERHELLO                    {SERVERSSL}}
        {SERVERSSL_SERVERCERT                     {SERVERSSL}}
        {SERVERSSL_HANDSHAKE                      {SERVERSSL}}
        {SERVERSSL_DATA                           {SERVERSSL}}
        {HTTP_REQUEST_SEND                        {HTTP}}
        {HTTP_REQUEST_RELEASE                     {HTTP}}
        {SERVER_DATA                              {}}
        {DNS_RESPONSE                             {DNS}}
        {SIP_REQUEST_SEND                         {SIP}}
        {SIP_RESPONSE                             {SIP}}
        {SIP_RESPONSE_SEND                        {SIP}}
        {HTTP_RESPONSE                            {FASTHTTP HTTP}}
        {HTTP_RESPONSE_DATA                       {HTTP}}
        {HTTP_RESPONSE_CONTINUE                   {HTTP}}
        {ASM_RESPONSE_VIOLATION                   {ASM}}
        {ASM_RESPONSE_LOGIN                       {ASM}}
        {BOTDEFENSE_REQUEST                       {BOTDEFENSE}}
        {BOTDEFENSE_ACTION                        {BOTDEFENSE}}
        {CACHE_UPDATE                             {CACHE WEBACCELERATION}}
        {STREAM_MATCHED                           {STREAM}}
        {HTML_TAG_MATCHED                         {HTML}}
        {HTML_COMMENT_MATCHED                     {HTML}}
        {REWRITE_RESPONSE_DONE                    {REWRITE}}
        {HTTP_RESPONSE_RELEASE                    {HTTP}}
        {HTTP_DISABLED                            {HTTP}}
        {HTTP_REJECT                              {HTTP}}
        {SERVER_CLOSED                            {}}
        {CLIENT_CLOSED                            {}}
        {ACCESS_SESSION_CLOSED                    {ACCESS}}
    }

    # Build index: event -> position in master ordering
    variable _event_index
    array set _event_index {}
    variable _idx 0
    foreach _entry $MASTER_ORDER {
        set _event_index([lindex $_entry 0]) $_idx
        incr _idx
    }
    unset _idx _entry

    # Events that fire at most once per connection
    variable ONCE_PER_CONNECTION {
        ACCESS_POLICY_AGENT_EVENT
        ACCESS_POLICY_COMPLETED
        ACCESS_SESSION_CLOSED
        ACCESS_SESSION_STARTED
        CLIENTSSL_CLIENTCERT
        CLIENTSSL_CLIENTHELLO
        CLIENTSSL_HANDSHAKE
        CLIENTSSL_PASSTHROUGH
        CLIENTSSL_SERVERHELLO_SEND
        CLIENT_ACCEPTED
        CLIENT_CLOSED
        FLOW_INIT
        RULE_INIT
    }

    # Events that fire once per HTTP transaction (repeatable on keep-alive)
    variable PER_REQUEST {
        ACCESS_ACL_ALLOWED
        ACCESS_ACL_DENIED
        ACCESS_PER_REQUEST_AGENT_EVENT
        ASM_REQUEST_BLOCKING
        ASM_REQUEST_DONE
        ASM_REQUEST_VIOLATION
        ASM_RESPONSE_VIOLATION
        BOTDEFENSE_ACTION
        BOTDEFENSE_REQUEST
        CACHE_REQUEST
        CACHE_RESPONSE
        CACHE_UPDATE
        DNS_REQUEST
        DNS_RESPONSE
        HTTP_CLASS_FAILED
        HTTP_CLASS_SELECTED
        HTTP_PROXY_REQUEST
        HTTP_REQUEST
        HTTP_REQUEST_DATA
        HTTP_REQUEST_RELEASE
        HTTP_REQUEST_SEND
        HTTP_RESPONSE
        HTTP_RESPONSE_CONTINUE
        HTTP_RESPONSE_DATA
        HTTP_RESPONSE_RELEASE
        IN_DOSL7_ATTACK
        LB_FAILED
        LB_QUEUED
        LB_SELECTED
        SA_PICKED
        SERVERSSL_CLIENTHELLO_SEND
        SERVERSSL_HANDSHAKE
        SERVERSSL_SERVERCERT
        SERVERSSL_SERVERHELLO
        SERVER_CLOSED
        SERVER_CONNECTED
        SERVER_INIT
        SIP_REQUEST
        SIP_REQUEST_SEND
        SIP_RESPONSE
        SIP_RESPONSE_SEND
        STREAM_MATCHED
    }

    # Pre-built flow chains
    #
    # Each chain: profiles + ordered steps {event phase}

    variable FLOW_CHAINS
    array set FLOW_CHAINS {}

    set FLOW_CHAINS(plain_tcp) {
        profiles {TCP}
        steps {
            {RULE_INIT init}
            {CLIENT_ACCEPTED l4_client}
            {CLIENT_DATA l4_client}
            {LB_SELECTED lb}
            {SA_PICKED lb}
            {SERVER_INIT lb}
            {SERVER_CONNECTED l4_server}
            {SERVER_DATA l4_server}
            {SERVER_CLOSED l4_teardown}
            {CLIENT_CLOSED l4_teardown}
        }
    }

    set FLOW_CHAINS(tcp_clientssl_http) {
        profiles {CLIENTSSL HTTP TCP}
        steps {
            {RULE_INIT init}
            {CLIENT_ACCEPTED l4_client}
            {CLIENTSSL_CLIENTHELLO tls_client}
            {CLIENTSSL_SERVERHELLO_SEND tls_client}
            {CLIENTSSL_CLIENTCERT tls_client}
            {CLIENTSSL_HANDSHAKE tls_client}
            {HTTP_REQUEST http_request}
            {HTTP_REQUEST_DATA http_request}
            {LB_SELECTED lb}
            {SA_PICKED lb}
            {SERVER_INIT lb}
            {SERVER_CONNECTED l4_server}
            {HTTP_REQUEST_SEND http_request_server}
            {HTTP_REQUEST_RELEASE http_request_server}
            {HTTP_RESPONSE http_response}
            {HTTP_RESPONSE_DATA http_response}
            {HTTP_RESPONSE_RELEASE http_response}
            {SERVER_CLOSED l4_teardown}
            {CLIENT_CLOSED l4_teardown}
        }
    }

    set FLOW_CHAINS(tcp_clientssl_serverssl_http) {
        profiles {CLIENTSSL HTTP SERVERSSL TCP}
        steps {
            {RULE_INIT init}
            {CLIENT_ACCEPTED l4_client}
            {CLIENTSSL_CLIENTHELLO tls_client}
            {CLIENTSSL_SERVERHELLO_SEND tls_client}
            {CLIENTSSL_CLIENTCERT tls_client}
            {CLIENTSSL_HANDSHAKE tls_client}
            {HTTP_REQUEST http_request}
            {HTTP_REQUEST_DATA http_request}
            {LB_SELECTED lb}
            {SA_PICKED lb}
            {SERVER_INIT lb}
            {SERVER_CONNECTED l4_server}
            {SERVERSSL_CLIENTHELLO_SEND tls_server}
            {SERVERSSL_SERVERHELLO tls_server}
            {SERVERSSL_SERVERCERT tls_server}
            {SERVERSSL_HANDSHAKE tls_server}
            {HTTP_REQUEST_SEND http_request_server}
            {HTTP_REQUEST_RELEASE http_request_server}
            {HTTP_RESPONSE http_response}
            {HTTP_RESPONSE_DATA http_response}
            {HTTP_RESPONSE_RELEASE http_response}
            {SERVER_CLOSED l4_teardown}
            {CLIENT_CLOSED l4_teardown}
        }
    }

    set FLOW_CHAINS(tcp_clientssl_serverssl_http_collect) {
        profiles {CLIENTSSL HTTP SERVERSSL TCP}
        steps {
            {RULE_INIT init}
            {CLIENT_ACCEPTED l4_client}
            {CLIENTSSL_CLIENTHELLO tls_client}
            {CLIENTSSL_SERVERHELLO_SEND tls_client}
            {CLIENTSSL_HANDSHAKE tls_client}
            {HTTP_REQUEST http_request}
            {HTTP_REQUEST_DATA http_request}
            {LB_SELECTED lb}
            {SA_PICKED lb}
            {SERVER_INIT lb}
            {SERVER_CONNECTED l4_server}
            {SERVERSSL_CLIENTHELLO_SEND tls_server}
            {SERVERSSL_SERVERHELLO tls_server}
            {SERVERSSL_SERVERCERT tls_server}
            {SERVERSSL_HANDSHAKE tls_server}
            {HTTP_REQUEST_SEND http_request_server}
            {HTTP_REQUEST_RELEASE http_request_server}
            {HTTP_RESPONSE http_response}
            {HTTP_RESPONSE_DATA http_response}
            {HTTP_RESPONSE_RELEASE http_response}
            {SERVER_CLOSED l4_teardown}
            {CLIENT_CLOSED l4_teardown}
        }
    }

    set FLOW_CHAINS(tcp_dns) {
        profiles {DNS TCP}
        steps {
            {RULE_INIT init}
            {CLIENT_ACCEPTED l4_client}
            {CLIENT_DATA l4_client}
            {DNS_REQUEST dns_request}
            {LB_SELECTED lb}
            {SA_PICKED lb}
            {SERVER_INIT lb}
            {SERVER_CONNECTED l4_server}
            {DNS_RESPONSE dns_response}
            {SERVER_CLOSED l4_teardown}
            {CLIENT_CLOSED l4_teardown}
        }
    }

    set FLOW_CHAINS(tcp_http) {
        profiles {HTTP TCP}
        steps {
            {RULE_INIT init}
            {CLIENT_ACCEPTED l4_client}
            {HTTP_REQUEST http_request}
            {HTTP_REQUEST_DATA http_request}
            {LB_SELECTED lb}
            {SA_PICKED lb}
            {SERVER_INIT lb}
            {SERVER_CONNECTED l4_server}
            {HTTP_REQUEST_SEND http_request_server}
            {HTTP_REQUEST_RELEASE http_request_server}
            {HTTP_RESPONSE http_response}
            {HTTP_RESPONSE_DATA http_response}
            {HTTP_RESPONSE_RELEASE http_response}
            {SERVER_CLOSED l4_teardown}
            {CLIENT_CLOSED l4_teardown}
        }
    }

    set FLOW_CHAINS(udp_dns) {
        profiles {DNS UDP}
        steps {
            {RULE_INIT init}
            {DNS_REQUEST dns_request}
            {LB_SELECTED lb}
            {DNS_RESPONSE dns_response}
        }
    }

}
