// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! snit types and widgets and `itcl::class` recorded as classes.

use super::*;

// snit (tcllib) type / widget support.  Verified against real
// `tclsh8.6` + tcllib.

#[test]
fn snit_type_recorded_as_class_with_members() {
    let src = "snit::type ::foo::Bar {\n\
                   variable v1\n\
                   typevariable tv1\n\
                   method m1 {a b} { return [expr {$a+$b+$v1}] }\n\
                   typemethod tm1 {} { return $tv1 }\n\
                   constructor {args} { set v1 0 }\n\
                   destructor { unset v1 }\n\
                   typeconstructor { set tv1 0 }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let c = r.all_classes.get("::foo::Bar").expect("Bar class recorded");
    assert_eq!(c.metaclass, "snit::type");
    assert!(c.methods.contains_key("m1"));
    assert!(c.class_methods.contains_key("tm1"));
    assert!(c.class_methods.contains_key("<typeconstructor>"));
    assert_eq!(c.constructors.len(), 1);
    assert!(c.destructor.is_some());
    assert_eq!(c.variables, vec!["v1".to_string(), "tv1".to_string()]);
}

#[test]
fn snit_widget_keeps_win_and_hull_vars() {
    // A snit::widget injects `win` and `hull` instance variables — both
    // are recorded (only the four implicit scalars are filtered).
    let src = "snit::widget Dial {\n\
                   variable state\n\
                   method draw {} { return $win }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let c = r.all_classes.get("::Dial").expect("Dial recorded");
    assert_eq!(c.metaclass, "snit::widget");
    assert_eq!(
        c.variables,
        vec!["win".to_string(), "hull".to_string(), "state".to_string()]
    );
}

#[test]
fn snit_method_body_suppresses_self_dispatch_and_implicit_vars() {
    // Inside a snit method, `$self`/`$component` dispatch and reads of
    // instance variables must not false-fire W307 (non-literal command),
    // W210 (read-before-set), or W211/W214 (unused).
    let src = "snit::widget mywidget {\n\
                   variable helper\n\
                   component inner\n\
                   method draw {} {\n\
                       $self configure -bg white\n\
                       $inner render\n\
                       $helper compute\n\
                       return $win\n\
                   }\n\
                   }";
    let r = crate::provider_fixtures::analyse(
        &format!("package require snit\n{src}"),
        "tcl8.6",
        &[crate::provider_fixtures::Provider::Snit],
    );
    for code in ["W210", "W211", "W214", "W307", "W308"] {
        assert!(
            !r.diagnostics.iter().any(|d| d.code.as_str() == code),
            "{code} must not fire in a snit method body: {:?}",
            r.diagnostics
        );
    }
}

#[test]
fn snit_widgetadaptor_and_qualified_definer() {
    let mut a = Analyser::new();
    let r = a.analyse("::snit::widgetadaptor Foo { method m {} {} }", "tcl8.6");
    let c = r.all_classes.get("::Foo").expect("Foo recorded");
    assert_eq!(c.metaclass, "::snit::widgetadaptor");
    assert!(c.methods.contains_key("m"));
}

#[test]
fn non_snit_command_is_not_a_class() {
    // A plain command that merely starts with `snit` is not a definer.
    let mut a = Analyser::new();
    let r = a.analyse("snitch foo { bar }", "tcl8.6");
    assert_eq!(r.all_classes.len(), 0);
}

// [incr Tcl] `itcl::class` — recorded as a `ClassDef` with method scopes,
// access modifiers unwrapped, `inherit` → superclasses, `this` implicit.

#[test]
fn itcl_class_recorded_with_members() {
    let src = "itcl::class ::widgets::Dial {\n\
                   inherit Base\n\
                   variable value 0\n\
                   common count 0\n\
                   constructor {args} { set value 0 }\n\
                   destructor { unset value }\n\
                   method spin {delta} { incr value $delta }\n\
                   proc reset {} { set count 0 }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let c = r
        .all_classes
        .get("::widgets::Dial")
        .expect("Dial class recorded");
    assert_eq!(c.metaclass, "itcl::class");
    assert_eq!(c.superclasses, vec!["Base".to_string()]);
    assert!(c.methods.contains_key("spin"));
    assert!(
        c.class_methods.contains_key("reset"),
        "proc is a class method"
    );
    assert_eq!(c.constructors.len(), 1);
    assert!(c.destructor.is_some());
    // `variable` + `common` declarations are recorded (implicit `this` is
    // filtered).
    assert_eq!(c.variables, vec!["value".to_string(), "count".to_string()]);
}

#[test]
fn itcl_access_modifiers_unwrap_with_visibility() {
    let src = "itcl::class C {\n\
                   public method api {} {}\n\
                   private method helper {} {}\n\
                   protected variable state 0\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    let c = r.all_classes.get("::C").expect("C recorded");
    assert_eq!(c.methods["api"].visibility, "public");
    assert_eq!(c.methods["helper"].visibility, "private");
    // The wrapped `protected variable state` is still a declared variable.
    assert!(c.variables.contains(&"state".to_string()));
}

#[test]
fn itcl_method_body_suppresses_this_and_instance_vars() {
    // Inside an itcl method, `$this` dispatch and reads of instance /
    // `common` variables must not false-fire W210/W211/W307.
    let src = "itcl::class C {\n\
                   variable handler\n\
                   common registry\n\
                   method run {} {\n\
                       $this configure\n\
                       $handler process\n\
                       return $registry\n\
                   }\n\
                   }";
    let mut a = Analyser::new();
    let r = a.analyse(src, "tcl8.6");
    for code in ["W210", "W211", "W214", "W307", "W308"] {
        assert!(
            !r.diagnostics.iter().any(|d| d.code.as_str() == code),
            "{code} must not fire in an itcl method body: {:?}",
            r.diagnostics
        );
    }
}

#[test]
fn non_itcl_command_is_not_a_class() {
    // A command that merely starts with `itcl` is not a definer.
    let mut a = Analyser::new();
    let r = a.analyse("itclish foo { bar }", "tcl8.6");
    assert_eq!(r.all_classes.len(), 0);
}
