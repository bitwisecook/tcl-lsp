# KCS: feature — versioned BIG-IP profile defaults in reports

> **Audience:** User
> **Type:** Functionality

## Summary

The standalone BIG-IP report resolves profile fields omitted from an SCF by
using the defaults for that configuration's BIG-IP version.

## Applies to

tcl-lsp CLI

## Question

How does the BIG-IP report handle profile fields that are absent from an SCF?

## How to use

Generate a report from an SCF or UCS in the usual way. An SCF written by BIG-IP
normally starts with a version marker such as:

```text
#TMSH-VERSION: 21.1.0.1
```

The report reads that marker and selects the matching version band from its
embedded `/config/profile_base.conf` catalogue. Explicit fields on a custom
profile take precedence over those base defaults. This matters because TMSH
normally omits values that are unchanged from the base profile.

BIG-IP 21.1 coverage includes:

- the canonical Client SSL and Server SSL profiles using the
  `/Common/f5-default` cipher group;
- TLS 1.0 and TLS 1.1 disabled in those canonical SSL profiles;
- the AIMCP profile introduced in 21.1;
- JSON and SSE profile limits introduced in BIG-IP 21.x; and
- the MCP persistence type and its `mcp-encryption-passphrase` field.

If the version marker is absent or cannot be parsed, the report uses the newest
embedded defaults.

## Software support lifecycle

The same version marker selects the BIG-IP software-branch lifecycle published
in F5 [K5903](https://my.f5.com/manage/s/article/K5903). Each device header
shows:

- the branch's first-customer-ship date;
- End of Software Development (EoSD);
- End of Technical Support (EoTS), which completes the software branch's End
  of Life (EoL); and
- the date of the embedded K5903 policy snapshot.

The report raises an amber warning when EoSD/EoTS is within 365 days and a red
warning after a milestone has passed. Reports remain fully self-contained: the
schedule is compiled into the generator and no request is made to F5 while a
report is opened. Use the included K5903 link to verify that the policy has not
changed. Hardware and FIPS variants can have independent dates, so also check
K5903 and K9476 before planning an upgrade.

## Example

This profile does not repeat its inherited cipher settings:

```tcl
#TMSH-VERSION: 21.1.0.1
ltm profile client-ssl /Common/application_tls {
    defaults-from /Common/clientssl
}
```

The report displays `/Common/f5-default` for its ciphers. The same SCF marked
as BIG-IP 20.1 displays `DEFAULT`, matching the older canonical Client SSL
profile.
