#!/usr/bin/env python3
"""Generate inputs.tsv: every BIG-IP name/address incantation probed for Q8.

Columns: id, group, input, doc
  group  names | v4 | v6 | port | fqdn | member | net | rest | gtm | combo
  doc    the documented BIG-IP reading where one is known (tmsh/bigip.conf),
         `?` where it needs the oracle (Q1), `bad` for inputs BIG-IP rejects
         (per F5 documentation; unverified on a device unless the design page
         cites a repo sample).
Run: python3 -I gen_inputs.py > inputs.tsv
"""

HAND = [
    # --- names and paths -------------------------------------------------
    ("names", "/Common/name", "partition=Common name=name"),
    ("names", "/Common/folder/sub/name", "partition=Common folders=folder/sub name=name"),
    ("names", "/Tenant/name", "partition=Tenant name=name"),
    ("names", "/Tenant_A/app/name", "partition=Tenant_A folders=app name=name"),
    ("names", "/Common/app.app/app_vs", "partition=Common folders=app.app(iApp) name=app_vs"),
    ("names", "/Common/app.app/app_pool", "partition=Common folders=app.app(iApp) name=app_pool"),
    ("names", "/Tenant/Shared/pool1", "AS3 Shared folder"),
    ("names", "name", "relative; partition from context"),
    ("names", "folder/name", "relative folder path; ? (Q1)"),
    ("names", "/Common/my name", "space: legal only quoted in tmsh (unverified)"),
    ('names', '/Common/q"t', "bad (quote) (unverified)"),
    ("names", "/Common/{b}", "bad (brace) (unverified)"),
    ("names", "/Common/a.b", "dot legal"),
    ("names", "/Common/a:b", "colon legal in names? ? (Q1); clashes with member:port"),
    ("names", "/Common/a%b", "percent: ? (Q1); clashes with %rd"),
    ("names", "/Common/a@b", "? (Q1)"),
    ("names", "/Common/a-b", "legal"),
    ("names", "/Common/a_b", "legal"),
    ("names", "/Common/10.0.0.1", "node named after its address"),
    ("names", "/Common/10.0.0.1%2", "node named after its RD address"),
    ("names", "/Common/2001:db8::1", "IPv6 node name"),
    ("names", "/Common/web.example.com", "FQDN node name"),
    ("names", "/Common/", "bad"),
    ("names", "/Common//name", "bad (empty segment)"),
    ("names", "//Common/name", "bad"),
    ("names", "/common/name", "partition names case-sensitive (unverified)"),
    ("names", "/COMMON/name", "distinct partition (unverified)"),
    ("names", "/", "bad"),
    ("names", "", "bad"),
    ("names", "Common/name", "relative folder Common/name, not /Common (unverified)"),
    ("names", "/Common", "partition alone (folder)"),
    ("names", "/Common/name/", "bad (trailing slash) (unverified)"),
    ("names", "/Common/a/b/c/d/name", "deep folders"),
    ("names", "/Common/ünïcode", "non-ASCII: ? (Q1)"),
    ("names", "/Common/Drafts/policy1", "LTM policy draft folder"),
    ("names", "/Common/.hidden", "? (Q1)"),
    ("names", "/Common/name ", "trailing space"),
    ("names", "/Tenant/app.app/sub/name", "iApp folder with subfolder"),
    # --- REST ----------------------------------------------------------------
    ("rest", "~Common~name", "REST /Common/name"),
    ("rest", "~Common~folder~name", "REST /Common/folder/name"),
    ("rest", "~Common~app.app~app_vs", "REST iApp"),
    ("rest", "~Tenant~name", "REST /Tenant/name"),
    ("rest", "~Common~10.0.0.1%252", "REST node 10.0.0.1%2, % URL-encoded"),
    ("rest", "~Common~10.0.0.1%2", "REST node, raw % (broken URL)"),
    ("rest", "~Common~2001:db8::1", "REST IPv6 node"),
    ("rest", "~Common~my%20name", "REST space"),
    # --- IPv4 ----------------------------------------------------------------
    ("v4", "10.0.0.1", "v4"),
    ("v4", "10.0.0.1%2", "v4 rd=2"),
    ("v4", "10.0.0.1%0", "v4 rd=0 (default RD; tmsh omits %0)"),
    ("v4", "10.0.0.1%65534", "rd max (unverified)"),
    ("v4", "10.0.0.1%65535", "bad: rd out of range (unverified)"),
    ("v4", "10.0.0.1%99999999999", "bad"),
    ("v4", "10.0.0.1%-1", "bad"),
    ("v4", "10.0.0.1%", "bad"),
    ("v4", "10.0.0.1%a", "bad (rd is numeric in config; name in tmsh cli? unverified)"),
    ("v4", "10.0.0.1%2%3", "bad"),
    ("v4", "10.0.0.1:80", "v4 port 80"),
    ("v4", "10.0.0.1%2:80", "v4 rd=2 port 80"),
    ("v4", "10.0.0.1:0", "port 0 = any"),
    ("v4", "10.0.0.1:any", "port any (tmsh input)"),
    ("v4", "10.0.0.1:http", "service name (tmsh shows names unless numeric)"),
    ("v4", "10.0.0.1:https", "service name"),
    ("v4", "10.0.0.1:ssh", "service name"),
    ("v4", "10.0.0.1:domain", "service name 53"),
    ("v4", "10.0.0.1:65535", "port max"),
    ("v4", "10.0.0.1:65536", "bad"),
    ("v4", "10.0.0.1:-1", "bad"),
    ("v4", "10.0.0.1:*", "monitor wildcard"),
    ("v4", "0.0.0.0", "wildcard address"),
    ("v4", "0.0.0.0:0", "wildcard; real 15.1 sample /Common/0.0.0.0:0"),
    ("v4", "0.0.0.0:any", "wildcard tmsh input"),
    ("v4", "0.0.0.0%2:0", "wildcard rd 2"),
    ("v4", "any", "address any (virtual-address; real sample)"),
    ("v4", "any:any", "registry default of ltm virtual destination"),
    ("v4", "any:0", "?"),
    ("v4", "*:*", "monitor destination (real sample)"),
    ("v4", "*:80", "monitor alias port only"),
    ("v4", "*", "?"),
    ("v4", "10.0.0.0/8", "prefix"),
    ("v4", "10.0.0.0/255.0.0.0", "dotted mask"),
    ("v4", "10.0.0.0%2/8", "rd + prefix (self-ip/route form)"),
    ("v4", "10.0.0.1%2/24", "self-ip form"),
    ("v4", "10.0.0.0/33", "bad"),
    ("v4", "10.0.0.0/255.0.255.0", "bad (non-contiguous)"),
    ("v4", "10.0.0.1/24", "host bits set (self-ip legit)"),
    ("v4", "010.0.0.1", "leading zero: ? (Q1)"),
    ("v4", "10.0.0.01", "leading zero: ? (Q1)"),
    ("v4", "256.0.0.1", "bad"),
    ("v4", "10.0.0", "bad"),
    ("v4", "10.0.0.1.80", "bad (v4 uses :)"),
    ("v4", "10.0.0.1 255.255.255.0", "addr mask (legacy class form)"),
    ("v4", "default", "route network default"),
    ("v4", "default%2", "route default in rd 2"),
    ("v4", "default-inet6", "route v6 default"),
    ("v4", "0.0.0.0/0", "source any (real sample)"),
    ("v4", "10.0.0.1-10.0.0.9", "range (AFM address list)"),
    ("v4", "255.255.255.255", "mask/broadcast (real sample mask)"),
    ("v4", "none", "mask none (real sample)"),
    # --- IPv6 ----------------------------------------------------------------
    ("v6", "2001:db8::1", "v6"),
    ("v6", "2001:0db8:0000:0000:0000:0000:0000:0001", "v6 expanded"),
    ("v6", "2001:DB8::1", "v6 upper"),
    ("v6", "::", "v6 unspecified"),
    ("v6", "::1", "loopback"),
    ("v6", "::ffff:10.0.0.1", "v4-mapped"),
    ("v6", "::ffff:10.0.0.1.80", "v4-mapped port 80"),
    ("v6", "fe80::1%eth0", "bad on BIG-IP (zone id is an OS form)"),
    ("v6", "fe80::1%1", "link-local rd 1"),
    ("v6", "fe80::1%1.80", "link-local rd 1 port 80"),
    ("v6", "2001:db8::1.80", "v6 port 80 (tmsh form)"),
    ("v6", "2001:db8::1%2.80", "v6 rd2 port 80"),
    ("v6", "2001:db8::1:80", "a full v6 address (no port)"),
    # Legacy colon-port for IPv6 (user-reported, early releases, version and
    # context unknown, unverified): each is ALSO a valid bare IPv6 address.
    ("v6", "fe80::1:80", "AMBIGUOUS: address fe80::1:80 | legacy fe80::1 port 80 (user-reported)"),
    ("v6", "2001:db8::1:443", "AMBIGUOUS: address | legacy 2001:db8::1 port 443"),
    ("v6", "2001:db8::1%2:80", "AMBIGUOUS: address%2 | legacy 2001:db8::1%2 port 80; tmsh would be .80"),
    ("v6", "fe80::1%1:80", "AMBIGUOUS legacy colon-port with rd"),
    ("v6", "::1:80", "AMBIGUOUS: ::1:80 address | ::1 port 80"),
    ("v6", "/Common/fe80::1:80", "AMBIGUOUS in a path"),
    ("v6", "/Common/2001:db8::1%2:80", "AMBIGUOUS in a path with rd"),
    ("v6", "2001:db8:0:0:0:0:1:80", "8 groups: only an address (no room for a port)"),
    ("v6", "2001:db8::1.http", "v6 service name"),
    ("v6", "2001:db8::1.any", "v6 port any"),
    ("v6", "2001:db8::1.0", "v6 port 0"),
    ("v6", "2001:db8::1.65536", "bad"),
    ("v6", "[2001:db8::1]:80", "URL/Host form, not tmsh"),
    ("v6", "[2001:db8::1%2]:80", "URL-ish with rd, not tmsh"),
    ("v6", "[2001:db8::1]%2:80", "bad"),
    ("v6", "[fe80::1]:80", "URL form"),
    ("v6", "[2001:db8::1].80", "not tmsh"),
    ("v6", "[2001:db8::1]", "URL host"),
    ("v6", "any6", "v6 wildcard"),
    ("v6", "any6.any", "v6 wildcard listener"),
    ("v6", "any6.0", "v6 wildcard port 0"),
    ("v6", "::.0", "v6 wildcard port 0"),
    ("v6", "::.any", "v6 wildcard port any"),
    ("v6", "*.*", "v6 monitor alias wildcard (unverified)"),
    ("v6", "2001:db8::/32", "v6 prefix"),
    ("v6", "2001:db8::%2/32", "v6 rd prefix"),
    ("v6", "2001:db8::1/128", "host"),
    ("v6", "::/0", "v6 any net"),
    ("v6", "::1.80", "loopback port 80 (ambiguous-looking)"),
    ("v6", "1::1.80", "v6 port 80"),
    ("v6", "2001:db8:::1", "bad"),
    ("v6", "2001:db8::g", "bad"),
    ("v6", "2001:db8::1::", "bad"),
    ("v6", "2001:db8::1/129", "bad"),
    # --- FQDN ----------------------------------------------------------------
    ("fqdn", "example.com", "fqdn"),
    ("fqdn", "example.com.", "fqdn trailing dot: ? (Q1)"),
    ("fqdn", "www.example.com:443", "fqdn member port"),
    ("fqdn", "xn--bcher-kva.example", "IDN a-label"),
    ("fqdn", "bücher.example", "IDN u-label: ? (Q1)"),
    ("fqdn", "*.example.com", "wildcard (GTM wide IP allows; node no)"),
    ("fqdn", "-bad.example", "bad"),
    ("fqdn", "a..b", "bad"),
    ("fqdn", "localhost", "single label: ? (Q1)"),
    ("fqdn", "host_name.example.com", "underscore: ? (Q1)"),
    ("fqdn", "10.0.0.1.nip.io", "fqdn with digits"),
    ("fqdn", "1.2.3.4.5", "bad-ish"),
    ("fqdn", "example.com%2:80", "fqdn with rd: ? (Q1)"),
    # --- pool / snat members ----------------------------------------------------
    ("member", "/Common/node1:80", "node1 port 80 (real sample shape)"),
    ("member", "/Common/node1:http", "service-name member"),
    ("member", "node1:80", "relative member"),
    ("member", "/Common/10.0.0.1:80", "member of address-named node"),
    ("member", "/Common/10.0.0.1%2:80", "member rd2"),
    ("member", "/Common/2001:db8::1.80", "v6 member"),
    ("member", "/Common/2001:db8::1%2.80", "v6 rd member"),
    ("member", "/Common/app.app/node1:80", "iApp member"),
    ("member", "/Common/www.example.com:443", "fqdn member"),
    ("member", "/Common/node1:0", "member port 0"),
    ("member", "/Common/node1:any", "member port any"),
    ("member", "/Common/node1", "snatpool member (no port)"),
    ("member", "/Common/a:b:80", "node named a:b ?"),
    ("member", "/Common/::ffff:10.0.0.1.80", "v4-mapped member"),
    ("member", "/Tenant/app/10.1.1.1%3:8080", "AS3 shape"),
    # --- GTM -----------------------------------------------------------------
    ("gtm", "/Common/server1:vs1", "gtm pool member server:vs"),
    ("gtm", "/Common/server1:/Common/vs1", "gtm pool member server:/vs path"),
    ("gtm", "/Common/dc1/server1:vs_name", "?"),
    ("gtm", "www.example.com", "wide IP"),
    ("gtm", "/Common/www.example.com", "wide IP full path"),
    ("gtm", "/Common/*.example.com", "wildcard wide IP"),
    # --- networks ------------------------------------------------------------
    ("net", "10.1.0.0%2/16", "route network rd"),
    ("net", "default%0", "default rd0"),
    ("net", "default-inet6%2", "v6 default rd2"),
    ("net", "any6", "?"),
    ("net", "10.0.0.0/8 ", "trailing space"),
]


def combos():
    addrs = [
        ("node1", "name"),
        ("www.example.com", "fqdn"),
        ("10.0.0.1", "v4"),
        ("0.0.0.0", "v4"),
        ("2001:db8::1", "v6"),
        ("::", "v6"),
        ("::ffff:10.0.0.1", "v6"),
        ("fe80::1", "v6"),
    ]
    rds = ["", "%0", "%2"]
    prefixes = ["", "/Common/", "/Common/app.app/", "/T1/"]
    for a, fam in addrs:
        sep = "." if fam == "v6" else ":"
        other = ":" if sep == "." else "."
        ports = ["", sep + "80", other + "80", sep + "any", sep + "http", sep + "0"]
        for rd in rds:
            for port in ports:
                for p in prefixes:
                    s = f"{p}{a}{rd}{port}"
                    note = f"combo fam={fam} rd={rd or '-'} port={port or '-'}"
                    if port.startswith(other) and port:
                        note += " WRONG-SEPARATOR"
                    yield ("combo", s, note)


def main():
    rows = list(HAND) + list(combos())
    seen = set()
    print("id\tgroup\tinput\tdoc")
    n = 0
    for g, s, d in rows:
        if s in seen:
            continue
        seen.add(s)
        n += 1
        print(f"i{n:03d}\t{g}\t{s}\t{d}")


if __name__ == "__main__":
    main()
