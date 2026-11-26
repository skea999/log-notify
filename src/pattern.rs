//! Pattern compilation and notification templates.
//!
//! Each source regex is compiled once at config load. Templates accept
//! `{line}` and named capture groups. `{{` and `}}` produce literal braces.

use std::collections::HashSet;

use regex::{Captures, Regex};
use thiserror::Error;

/// A compiled source pattern and its notification templates.
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    regex: Regex,
    summary: Template,
    body: Template,
}

/// Rendered notification text for one match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    /// Notification title.
    pub summary: String,
    /// Notification body.
    pub body: String,
}

/// Pattern or template compilation failure.
#[derive(Debug, Error)]
pub enum PatternError {
    /// The regex is invalid.
    #[error("invalid regex: {0}")]
    Regex(#[source] regex::Error),
    /// A capture group uses the reserved `line` name.
    #[error("capture group `{name}` conflicts with the reserved `{{line}}` placeholder")]
    ReservedCapture {
        /// Capture group name.
        name: String,
    },
    /// A template is malformed or references an unknown placeholder.
    #[error("{field} template: {message}")]
    Template {
        /// Template field name: `summary` or `body`.
        field: &'static str,
        /// Human-readable reason.
        message: String,
    },
}

impl CompiledPattern {
    /// Compile a regex and validate both templates against its capture names.
    pub fn compile(spec: &str, summary: &str, body: &str) -> Result<Self, PatternError> {
        let regex = Regex::new(spec).map_err(PatternError::Regex)?;

        let mut names = HashSet::new();
        for name in regex.capture_names().flatten() {
            if name == "line" {
                return Err(PatternError::ReservedCapture {
                    name: name.to_owned(),
                });
            }
            names.insert(name.to_owned());
        }

        let summary = Template::compile(summary, "summary", &names)?;
        let body = Template::compile(body, "body", &names)?;

        Ok(Self {
            regex,
            summary,
            body,
        })
    }

    /// Match a line. Returns `None` when the pattern does not match.
    #[must_use]
    pub fn captures<'h>(&self, line: &'h str) -> Option<Captures<'h>> {
        self.regex.captures(line)
    }

    /// Render the notification text for a match.
    #[must_use]
    pub fn render(&self, line: &str, captures: &Captures<'_>) -> Rendered {
        Rendered {
            summary: self.summary.render(line, captures),
            body: self.body.render(line, captures),
        }
    }
}

#[derive(Debug, Clone)]
struct Template {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone)]
enum Segment {
    Literal(String),
    Line,
    Capture(String),
}

impl Template {
    fn compile(
        input: &str,
        field: &'static str,
        names: &HashSet<String>,
    ) -> Result<Self, PatternError> {
        let mut segments = Vec::new();
        let mut literal = String::new();
        let mut chars = input.char_indices().peekable();

        while let Some((start, character)) = chars.next() {
            match character {
                '{' => {
                    if let Some((_, '{')) = chars.peek() {
                        chars.next();
                        literal.push('{');
                        continue;
                    }

                    let Some(end) = placeholder_end(&mut chars, field)? else {
                        return Err(template_error(
                            field,
                            "unclosed `{`; use `{{` for a literal brace",
                        ));
                    };
                    let name = input.get(start + 1..end).ok_or_else(|| {
                        template_error(field, "placeholder is not on a character boundary")
                    })?;
                    if name.is_empty() {
                        return Err(template_error(field, "empty placeholder `{}`"));
                    }
                    if !literal.is_empty() {
                        segments.push(Segment::Literal(std::mem::take(&mut literal)));
                    }
                    if name == "line" {
                        segments.push(Segment::Line);
                    } else if names.contains(name) {
                        segments.push(Segment::Capture(name.to_owned()));
                    } else {
                        return Err(template_error(
                            field,
                            format!("unknown placeholder `{{{name}}}`"),
                        ));
                    }
                }
                '}' => {
                    if let Some((_, '}')) = chars.peek() {
                        chars.next();
                        literal.push('}');
                    } else {
                        return Err(template_error(
                            field,
                            "unmatched `}`; use `}}` for a literal brace",
                        ));
                    }
                }
                _ => literal.push(character),
            }
        }

        if !literal.is_empty() {
            segments.push(Segment::Literal(literal));
        }

        Ok(Self { segments })
    }

    fn render(&self, line: &str, captures: &Captures<'_>) -> String {
        let mut output = String::new();
        for segment in &self.segments {
            match segment {
                Segment::Literal(value) => output.push_str(value),
                Segment::Line => output.push_str(line),
                Segment::Capture(name) => {
                    if let Some(value) = captures.name(name) {
                        output.push_str(value.as_str());
                    }
                }
            }
        }
        output
    }
}

fn placeholder_end(
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
    field: &'static str,
) -> Result<Option<usize>, PatternError> {
    for (index, character) in chars.by_ref() {
        match character {
            '}' => return Ok(Some(index)),
            '{' => {
                return Err(template_error(
                    field,
                    "nested `{` in placeholder; use `{{` for a literal brace",
                ));
            }
            _ => {}
        }
    }
    Ok(None)
}

fn template_error(field: &'static str, message: impl Into<String>) -> PatternError {
    PatternError::Template {
        field,
        message: message.into(),
    }
}
