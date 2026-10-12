# Fixed ASCII LF input. Public interpreter results only.
set ::observed {}
proc ::resolution_scope_handler {args} {
    lappend ::observed GLOBAL
    return handled
}
set nsInstallCode [catch {namespace unknown resolution_scope_handler} nsInstallResult]
puts [list namespace_install $nsInstallCode $nsInstallResult]
set nsCreateCode [catch {
    namespace eval ::ResolutionTrigger {
        proc resolution_scope_handler {args} {
            lappend ::observed TRIGGER
            return handled
        }
    }
} nsCreateResult]
puts [list namespace_create $nsCreateCode $nsCreateResult]
set nsTriggerCode [catch {
    namespace eval ::ResolutionTrigger {resolution_scope_missing_command}
} nsTriggerResult]
puts [list namespace_trigger $nsTriggerCode $nsTriggerResult observed $::observed]
set ::observed {}
proc ::resolution_package_handler {args} {
    lappend ::observed GLOBAL
    return
}
set pkgInstallCode [catch {
    namespace eval ::ResolutionInstaller {
        proc resolution_package_handler {args} {
            lappend ::observed INSTALLER
            return
        }
        package unknown resolution_package_handler
    }
} pkgInstallResult]
puts [list package_install $pkgInstallCode $pkgInstallResult]
set pkgTriggerCode [catch {
    namespace eval ::ResolutionTrigger {
        proc resolution_package_handler {args} {
            lappend ::observed TRIGGER
            return
        }
        package require resolution_scope_missing_package
    }
} pkgTriggerResult]
puts [list package_trigger $pkgTriggerCode $pkgTriggerResult observed $::observed]
