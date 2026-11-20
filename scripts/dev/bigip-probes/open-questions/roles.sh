#!/bin/bash
# Restricted-role tmsh: what each role can see and run over a real SSH login.
#
# `su` from root does NOT apply the role (tmsh still treats the session as
# root), so every command here is run through an actual SSH login as the
# role user, authenticated with a public key installed for the run.
#
# Usage (off-box):
#   roles.sh <ssh-to-root-cmd> <pubkey-file> <ssh-as-user-cmd-prefix>
# The last argument is a command prefix that takes `user@host` and a remote
# command, e.g. "ssh -i key -o ...".
set -u
ROOT=$1 PUB=$2 USERSSH=$3
HOST=192.168.9.24
P=tcllsp_oq
ROLES="guest operator manager resource-admin"
CMDS=(
  "list ltm rule ${P}_role_absent"
  "show sys version"
  "show cli version"
  "create ltm rule ${P}_role_made"
  "delete ltm rule ${P}_role_made"
  "modify cli version active 17.5.0"
  "show cli version"
  "run cli script ${P}_role"
  "run util bash -c id"
  "list sys db systemauth.disablebash"
)

$ROOT "cat > /var/tmp/${P}_role.conf" <<'EOF'
cli script tcllsp_oq_role {
proc script::run {} {
    foreach {k cmd} {
        list_rule {tmsh::list ltm rule tcllsp_oq_role_absent}
        show_version {tmsh::show sys version}
        create_rule {tmsh::create ltm rule tcllsp_oq_role_script_made}
        delete_rule {tmsh::delete ltm rule tcllsp_oq_role_script_made}
        exec_id {exec id}
    } {
        set c [catch {eval $cmd} r]
        puts "ROLE-SCRIPT $k $c [string range [string map {"\n" " "} $r] 0 100]"
    }
}
}
EOF
$ROOT "tmsh list cli script ${P}_role >/dev/null 2>&1 && echo ABORT; tmsh load sys config merge file /var/tmp/${P}_role.conf >/dev/null && echo SCRIPT~~created"

for role in $ROLES; do
  u=${P}_$(echo $role | tr - _)
  # create the user with a random password it never uses, then a key
  $ROOT "tmsh list auth user $u >/dev/null 2>&1 && { echo ABORT~~$u exists; exit 1; }
    pw=\$(head -c 24 /dev/urandom | base64 | tr -dc A-Za-z0-9 | head -c 20)Aa1
    printf 'create auth user %s partition-access replace-all-with { all-partitions { role %s } } shell tmsh password %s\n' $u $role \"\$pw\" | tmsh >/dev/null 2>&1
    unset pw
    # new accounts start with an expired password, which blocks even key logins
    chage -d \$((\$(date +%s) / 86400)) $u
    h=\$(getent passwd $u | cut -d: -f6); mkdir -p \$h/.ssh; cat > \$h/.ssh/authorized_keys; chmod 700 \$h/.ssh; chmod 600 \$h/.ssh/authorized_keys
    chown -R $u \$h/.ssh
    echo USER~~$role~~\$(getent passwd $u | cut -d: -f3,7)" < "$PUB"
  for cmd in "${CMDS[@]}"; do
    out=$($USERSSH "$u@$HOST" "$cmd" 2>&1); rc=$?
    echo "ROLE~~$role~~$cmd~~rc=$rc~~$(echo "$out" | tr -s ' \n' ' ' | cut -c1-1200)"
  done
  $ROOT "tmsh delete ltm rule ${P}_role_made >/dev/null 2>&1; tmsh delete ltm rule ${P}_role_script_made >/dev/null 2>&1
    tmsh modify cli version active 21.1.0.1 >/dev/null 2>&1
    tmsh delete auth user $u >/dev/null 2>&1; tmsh list auth user $u >/dev/null 2>&1 && echo LEFTOVER~~$u
    rm -rf /home/$u"
done
$ROOT "tmsh delete cli script ${P}_role >/dev/null 2>&1; tmsh list cli script ${P}_role >/dev/null 2>&1 && echo LEFTOVER~~script
  for r in ${P}_role_made ${P}_role_script_made; do tmsh list ltm rule \$r >/dev/null 2>&1 && echo LEFTOVER~~\$r; done
  rm -f /var/tmp/${P}_role.conf; echo active=\$(tmsh show cli version | awk '/active/{print \$2}')"
