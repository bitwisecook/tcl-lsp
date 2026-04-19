# Example iApp APL (Application Presentation Language) file
# This demonstrates the various APL constructs for iApp templates.

#include "f5.apl_common"

# Define a reusable type
define choice yesno_choice {
    "Yes" => "yes",
    "No" => "no"
}

define choice lb_method {
    "Round Robin" => "round-robin",
    "Least Connections" => "least-connections-member",
    "Fastest" => "fastest-node"
}

section intro {
    message "Welcome to the example iApp template."
}

section basic {
    string virtual_addr default "0.0.0.0" required validator "IpAddress"
    string virtual_port default "443" required validator "PortNumber"
    choice protocol display "medium" default "tcp" {
        "TCP" => "tcp",
        "UDP" => "udp"
    }
    yesno use_snat default "yes"
    password ssl_passphrase display "large"
}

section pool {
    string pool_name required
    lb_method lb_algorithm
    table members {
        string addr required validator "IpAddress"
        string port default "80" validator "PortNumber"
        string connection_limit default "0" validator "NonNegativeNumber"
    }
    choice monitor display "large" default "http" {
        "HTTP" => "http",
        "HTTPS" => "https",
        "TCP" => "tcp"
    }
}

section ssl {
    optional ( basic.protocol == "tcp" ) {
        choice ssl_profile display "xlarge" {
            "Client SSL" => "clientssl",
            "Server SSL" => "serverssl",
            "Both" => "both"
        }
        editchoice cipher_string default "DEFAULT" display "xxlarge"
        string sni_hostname validator "FQDN"
    }
}

section advanced {
    optional ( basic.use_snat == "yes" ) {
        choice snat_type default "automap" {
            "Auto Map" => "automap",
            "SNAT Pool" => "snatpool",
            "None" => "none"
        }
    }
    multichoice persistence default "cookie" display "large" {
        "Cookie" => "cookie",
        "Source Address" => "source_addr",
        "SSL Session ID" => "ssl"
    }
    string irule_name
    noyes disable_arp
    enadis http_compression default "disabled"
}

text {
    intro "Introduction"
    intro.message "Welcome message for this template."
    basic "Basic Configuration"
    basic.virtual_addr "Virtual Server IP Address"
    basic.virtual_port "Virtual Server Port"
    basic.protocol "Protocol"
    basic.use_snat "Use SNAT?"
    basic.ssl_passphrase "SSL Certificate Passphrase"
    pool "Pool Configuration"
    pool.pool_name "Pool Name"
    pool.lb_algorithm "Load Balancing Method"
    pool.members "Pool Members"
    pool.members.addr "Member Address"
    pool.members.port "Member Port"
    pool.members.connection_limit "Connection Limit"
    pool.monitor "Health Monitor"
    ssl "SSL/TLS Settings"
    ssl.ssl_profile "SSL Profile Type"
    ssl.cipher_string "Cipher String"
    ssl.sni_hostname "SNI Hostname"
    advanced "Advanced Settings"
    advanced.snat_type "SNAT Type"
    advanced.persistence "Persistence Method"
    advanced.irule_name "iRule Name"
    advanced.disable_arp "Disable ARP"
    advanced.http_compression "HTTP Compression"
}

text "de-DE" {
    intro "Einführung"
    basic "Grundkonfiguration"
    basic.virtual_addr "IP-Adresse des virtuellen Servers"
    basic.virtual_port "Port des virtuellen Servers"
}
