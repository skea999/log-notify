//! Pattern and template engine tests.
#![allow(clippy::literal_string_with_formatting_args)]

use log_notify::pattern::{CompiledPattern, PatternError};

#[test]
fn renders_line_and_named_captures() {
    let pattern = CompiledPattern::compile(
        r"^(?P<user>\w+) (?P<action>\w+)$",
        "{user} {action}",
        "{line}",
    )
    .expect("valid pattern");
    let line = "alice login";
    let captures = pattern.captures(line).expect("match");
    let rendered = pattern.render(line, &captures);

    assert_eq!(rendered.summary, "alice login");
    assert_eq!(rendered.body, line);
}

#[test]
fn optional_capture_renders_empty() {
    let pattern = CompiledPattern::compile(
        r"^(?P<user>\w+)(?: (?P<action>\w+))?$",
        "{user}:{action}",
        "",
    )
    .expect("valid pattern");
    let line = "alice";
    let captures = pattern.captures(line).expect("match");
    let rendered = pattern.render(line, &captures);

    assert_eq!(rendered.summary, "alice:");
    assert_eq!(rendered.body, "");
}

#[test]
fn no_match_returns_none() {
    let pattern = CompiledPattern::compile("FOUND$", "{line}", "{line}").expect("valid pattern");

    assert!(pattern.captures("nothing here").is_none());
}

#[test]
fn literal_braces_are_escaped() {
    let pattern = CompiledPattern::compile(r"^(?P<user>\w+)$", "{{line}} {user}", "}}")
        .expect("valid pattern");
    let line = "alice";
    let captures = pattern.captures(line).expect("match");
    let rendered = pattern.render(line, &captures);

    assert_eq!(rendered.summary, "{line} alice");
    assert_eq!(rendered.body, "}");
}

#[test]
fn invalid_regex_is_rejected() {
    let err = CompiledPattern::compile("(", "{line}", "").expect_err("invalid regex");

    assert!(matches!(err, PatternError::Regex(_)));
}

#[test]
fn unknown_placeholder_is_rejected() {
    let err = CompiledPattern::compile(r"(?P<user>\w+)", "{missing}", "")
        .expect_err("unknown placeholder");

    assert!(matches!(
        err,
        PatternError::Template {
            field: "summary",
            ..
        }
    ));
    assert!(format!("{err}").contains("unknown placeholder"));
}

#[test]
fn unclosed_brace_is_rejected() {
    let err = CompiledPattern::compile("x", "{user", "").expect_err("unclosed brace");

    assert!(matches!(
        err,
        PatternError::Template {
            field: "summary",
            ..
        }
    ));
    assert!(format!("{err}").contains("unclosed"));
}

#[test]
fn unmatched_close_brace_is_rejected() {
    let err = CompiledPattern::compile("x", "user}", "").expect_err("unmatched brace");

    assert!(matches!(
        err,
        PatternError::Template {
            field: "summary",
            ..
        }
    ));
    assert!(format!("{err}").contains("unmatched"));
}

#[test]
fn nested_brace_is_rejected() {
    let err = CompiledPattern::compile("x", "{user{name}}", "").expect_err("nested brace");

    assert!(matches!(err, PatternError::Template { .. }));
    assert!(format!("{err}").contains("nested"));
}

#[test]
fn empty_placeholder_is_rejected() {
    let err = CompiledPattern::compile("x", "{}", "").expect_err("empty placeholder");

    assert!(matches!(err, PatternError::Template { .. }));
    assert!(format!("{err}").contains("empty placeholder"));
}

#[test]
fn reserved_line_capture_is_rejected() {
    let err = CompiledPattern::compile(r"(?P<line>\w+)", "{line}", "").expect_err("reserved name");

    assert!(matches!(
        err,
        PatternError::ReservedCapture { ref name } if name == "line"
    ));
}

#[test]
fn body_template_errors_point_at_body() {
    let err = CompiledPattern::compile("x", "ok", "{missing}").expect_err("body placeholder");

    assert!(matches!(err, PatternError::Template { field: "body", .. }));
}
