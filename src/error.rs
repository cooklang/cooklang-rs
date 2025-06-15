//! Error type, formatting and utilities.

use serde::Serialize;
use std::{borrow::Cow, panic::RefUnwindSafe};

use crate::Span;

/// Handy label creation for [`SourceDiag`]
macro_rules! label {
    ($span:expr $(,)?) => {
        ($span.to_owned().into(), None)
    };
    ($span:expr, $message:expr $(,)?) => {
        ($span.to_owned().into(), Some($message.into()))
    };
    ($span:expr, $fmt:literal, $($arg:expr),+) => {
        label!($span, format!($fmt, $($arg),+))
    }
}
pub(crate) use label;

pub type CowStr = Cow<'static, str>;

/// A label is a pair of a code location and an optional hint at that location
pub type Label = (Span, Option<CowStr>);

/// A diagnostic of source code
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct SourceDiag {
    /// If the diagnostic is an error or warning
    pub severity: Severity,
    /// In which parsing stage did this origined
    pub stage: Stage,
    /// Report message describing the problem
    pub message: CowStr,
    /// Lower level error that produced the problem, if any
    #[serde(skip_serializing)]
    source: Option<std::sync::Arc<dyn std::error::Error + Send + Sync + RefUnwindSafe + 'static>>,
    /// Spans of the code that helps the user find the error
    ///
    /// It should be ordered from high to low importance. The first is the
    /// main location of the error.
    pub labels: Vec<Label>,
    /// Additional hints for the user
    ///
    /// It should be ordered from high to low importance.
    pub hints: Vec<CowStr>,
}

impl std::fmt::Display for SourceDiag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

impl RichError for SourceDiag {
    fn labels(&self) -> Cow<[Label]> {
        self.labels.as_slice().into()
    }

    fn hints(&self) -> Cow<[CowStr]> {
        self.hints.as_slice().into()
    }

    fn severity(&self) -> Severity {
        self.severity
    }
}

impl std::error::Error for SourceDiag {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // idk why I can't .as_deref but I can do this
        match &self.source {
            Some(err) => Some(err),
            None => None,
        }
    }
}

impl PartialEq for SourceDiag {
    fn eq(&self, other: &Self) -> bool {
        self.severity == other.severity && self.message == other.message
    }
}

impl SourceDiag {
    /// Creates a new error
    pub(crate) fn error(message: impl Into<CowStr>, label: Label, stage: Stage) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            labels: vec![label],
            hints: vec![],
            source: None,
            stage,
        }
    }

    /// Creates a new warning
    pub(crate) fn warning(message: impl Into<CowStr>, label: Label, stage: Stage) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            labels: vec![label],
            hints: vec![],
            source: None,
            stage,
        }
    }

    /// Creates a new unlabeled diagnostic
    ///
    /// This means there's no error location
    pub(crate) fn unlabeled(message: impl Into<CowStr>, severity: Severity, stage: Stage) -> Self {
        Self {
            severity,
            stage,
            message: message.into(),
            source: None,
            labels: vec![],
            hints: vec![],
        }
    }

    /// Checks if the diagnostic is an error
    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Checks if the diagnostic is a warning
    pub fn is_warning(&self) -> bool {
        self.severity == Severity::Warning
    }

    /// Adds a new label
    pub(crate) fn label(mut self, label: Label) -> Self {
        self.add_label(label);
        self
    }
    /// Adds a new label
    pub(crate) fn add_label(&mut self, label: Label) -> &mut Self {
        self.labels.push(label);
        self
    }

    /// Adds a new hint
    pub(crate) fn hint(mut self, hint: impl Into<CowStr>) -> Self {
        self.add_hint(hint);
        self
    }
    /// Adds a new hint
    pub(crate) fn add_hint(&mut self, hint: impl Into<CowStr>) -> &mut Self {
        self.hints.push(hint.into());
        self
    }
    /// Sets the error source
    ///
    /// This is where [`std::error::Error::source`] get's the information
    pub(crate) fn set_source(
        mut self,
        source: impl std::error::Error + Send + Sync + RefUnwindSafe + 'static,
    ) -> Self {
        self.source = Some(std::sync::Arc::new(source));
        self
    }
}

/// Diagnostic severity
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum Severity {
    /// Fatal error
    Error,
    /// Non fatal warning
    Warning,
}

/// Stage where the diagnostic origined
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum Stage {
    /// Parse stage
    Parse,
    /// Analysis stage
    Analysis,
}

/// Errors and warnings container with fancy formatting
///
/// The [`Display`](std::fmt::Display) implementation is not fancy formatting,
/// use one of the print or write methods.
#[derive(Debug, Clone, Serialize)]
pub struct SourceReport {
    buf: Vec<SourceDiag>,
    severity: Option<Severity>,
}

impl SourceReport {
    pub(crate) fn empty() -> Self {
        Self {
            buf: Vec::new(),
            severity: None,
        }
    }

    pub(crate) fn push(&mut self, err: SourceDiag) {
        debug_assert!(self.severity.is_none() || self.severity.is_some_and(|s| err.severity == s));
        self.buf.push(err);
    }

    pub(crate) fn error(&mut self, w: SourceDiag) {
        debug_assert_eq!(w.severity, Severity::Error);
        self.push(w);
    }

    pub(crate) fn warn(&mut self, w: SourceDiag) {
        debug_assert_eq!(w.severity, Severity::Warning);
        self.push(w);
    }

    pub(crate) fn retain(&mut self, f: impl Fn(&SourceDiag) -> bool) {
        self.buf.retain(f)
    }

    pub(crate) fn set_severity(&mut self, severity: Option<Severity>) {
        debug_assert!(
            severity.is_none()
                || severity.is_some_and(|s| self.buf.iter().all(|e| e.severity == s))
        );
        self.severity = severity;
    }

    /// Returns the severity of this report.
    ///
    /// - `None` means any severity.
    /// - `Some(sev)` means all errors in the report are of severity `sev`.
    pub fn severity(&self) -> Option<&Severity> {
        self.severity.as_ref()
    }

    /// Iterate over all diagnostics
    pub fn iter(&self) -> impl Iterator<Item = &SourceDiag> {
        self.buf.iter()
    }

    /// Get the errors
    pub fn errors(&self) -> impl Iterator<Item = &SourceDiag> {
        self.iter().filter(|e| e.severity == Severity::Error)
    }

    /// Get the warnings
    pub fn warnings(&self) -> impl Iterator<Item = &SourceDiag> {
        self.iter().filter(|e| e.severity == Severity::Warning)
    }

    /// Check if the report has any error
    pub fn has_errors(&self) -> bool {
        match self.severity {
            Some(Severity::Warning) => false,
            Some(Severity::Error) => !self.buf.is_empty(),
            None => self.errors().next().is_some(),
        }
    }

    /// Check if the report has any warning
    pub fn has_warnings(&self) -> bool {
        match self.severity {
            Some(Severity::Warning) => !self.buf.is_empty(),
            Some(Severity::Error) => false,
            None => self.warnings().next().is_some(),
        }
    }

    /// Check if the report is empty
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Divide the report into two report, errors and warnings
    ///
    /// The first is the errors and the second, warnings
    pub fn unzip(self) -> (SourceReport, SourceReport) {
        let (errors, warnings) = self.buf.into_iter().partition(SourceDiag::is_error);
        (
            Self {
                buf: errors,
                severity: Some(Severity::Error),
            },
            Self {
                buf: warnings,
                severity: Some(Severity::Warning),
            },
        )
    }

    /// Removes the warnings leaving only errors
    pub fn remove_warnings(&mut self) {
        self.buf.retain(SourceDiag::is_error)
    }

    /// Consumes the report and returns [`Vec`] of [`SourceDiag`]
    pub fn into_vec(self) -> Vec<SourceDiag> {
        self.buf
    }

    /// Write a formatted report
    pub fn write(
        &self,
        file_name: &str,
        source_code: &str,
        color: bool,
        w: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        let lidx = codesnake::LineIndex::new(source_code);

        for err in self.warnings() {
            write_report(&mut *w, err, &lidx, file_name, color)?;
        }
        for err in self.errors() {
            write_report(&mut *w, err, &lidx, file_name, color)?;
        }
        Ok(())
    }
    /// Print a formatted report to stdout
    pub fn print(&self, file_name: &str, source_code: &str, color: bool) -> std::io::Result<()> {
        self.write(file_name, source_code, color, &mut std::io::stdout().lock())
    }
    /// Print a formatted report to stderr
    pub fn eprint(&self, file_name: &str, source_code: &str, color: bool) -> std::io::Result<()> {
        self.write(file_name, source_code, color, &mut std::io::stderr().lock())
    }
}

impl std::fmt::Display for SourceReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for err in self.iter() {
            err.fmt(f)?;
        }
        Ok(())
    }
}
impl std::error::Error for SourceReport {}

/// Output from the different passes of the parsing process
#[derive(Debug, Clone, Serialize)]
pub struct PassResult<T> {
    output: Option<T>,
    report: SourceReport,
}

impl<T> PassResult<T> {
    pub(crate) fn new(output: Option<T>, report: SourceReport) -> Self {
        Self { output, report }
    }

    /// Check if the result has any output. It may not be valid.
    pub fn has_output(&self) -> bool {
        self.output.is_some()
    }

    /// Get the report
    pub fn report(&self) -> &SourceReport {
        &self.report
    }

    /// Check if the result is valid.
    ///
    /// If the result is invalid, the output, if any, should be discarded or
    /// used knowing that it contains errors or be incomplete.
    pub fn is_valid(&self) -> bool {
        self.has_output() && !self.report.has_errors()
    }

    /// Get the output
    pub fn output(&self) -> Option<&T> {
        self.output.as_ref()
    }

    /// Get the output only if it's valid
    pub fn valid_output(&self) -> Option<&T> {
        if self.is_valid() {
            self.output()
        } else {
            None
        }
    }

    /// Transform into a common Rust [`Result`]
    ///
    /// If the result is valid, the [`Ok`] variant holds the output and a
    /// report with only warnings. Otherwise [`Err`] holds a report with the
    /// errors (and warnings).
    pub fn into_result(mut self) -> Result<(T, SourceReport), SourceReport> {
        if !self.is_valid() {
            return Err(self.report);
        }
        self.report.set_severity(Some(Severity::Warning));
        Ok((self.output.unwrap(), self.report))
    }

    /// Transform into a [`SourceReport`] discarding the output
    pub fn into_report(self) -> SourceReport {
        self.report
    }

    /// Transform into the output discarding errors/warnings
    pub fn into_output(self) -> Option<T> {
        self.output
    }

    /// Unwraps the inner output
    ///
    /// # Panics
    /// If the output is `None`.
    pub fn unwrap_output(self) -> T {
        self.output.unwrap()
    }

    /// Get output, errors and warnings in a tuple
    pub fn into_tuple(self) -> (Option<T>, SourceReport) {
        (self.output, self.report)
    }

    /// Map the inner output
    pub fn map<F, O>(self, f: F) -> PassResult<O>
    where
        F: FnOnce(T) -> O,
    {
        PassResult {
            output: self.output.map(f),
            report: self.report,
        }
    }
}

/// Trait to enhace errors with rich metadata
pub trait RichError: std::error::Error {
    fn labels(&self) -> Cow<[Label]> {
        Cow::Borrowed(&[])
    }
    fn hints(&self) -> Cow<[CowStr]> {
        Cow::Borrowed(&[])
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
}

/// Writes a rich error report
///
/// This function should not be used in a loop as each call will
/// perform a light parse of the whole source code.
pub fn write_rich_error(
    error: &dyn RichError,
    file_name: &str,
    source_code: &str,
    color: bool,
    w: impl std::io::Write,
) -> std::io::Result<()> {
    let lidx = codesnake::LineIndex::new(source_code);
    write_report(w, error, &lidx, file_name, color)
}

#[derive(Default)]
struct ColorGenerator(usize);

impl ColorGenerator {
    const COLORS: &'static [yansi::Color] = &[
        yansi::Color::BrightMagenta,
        yansi::Color::BrightGreen,
        yansi::Color::BrightCyan,
        yansi::Color::BrightBlue,
        yansi::Color::BrightGreen,
        yansi::Color::BrightYellow,
        yansi::Color::BrightRed,
    ];

    fn next(&mut self) -> yansi::Color {
        let c = Self::COLORS[self.0];
        if self.0 == Self::COLORS.len() - 1 {
            self.0 = 0;
        } else {
            self.0 += 1;
        }
        c
    }
}

fn write_report(
    mut w: impl std::io::Write,
    err: &dyn RichError,
    lidx: &codesnake::LineIndex,
    file_name: &str,
    color: bool,
) -> std::io::Result<()> {
    use yansi::Paint;

    let cond = yansi::Condition::cached(color);

    let sev_color = match err.severity() {
        Severity::Error => yansi::Color::Red,
        Severity::Warning => yansi::Color::Yellow,
    };
    match err.severity() {
        Severity::Error => writeln!(w, "{} {err}", "Error:".paint(sev_color).whenever(cond))?,
        Severity::Warning => writeln!(w, "{} {err}", "Warning:".paint(sev_color).whenever(cond))?,
    }
    if let Some(source) = err.source() {
        writeln!(w, "  {} {source}", "╰▶ ".paint(sev_color).whenever(cond))?;
    }

    let mut cg = ColorGenerator::default();

    let mut labels = err.labels();
    if !labels.is_empty() {
        // codesnake requires the labels to be sorted
        labels.to_mut().sort_unstable_by_key(|l| l.0);

        let mut colored_labels = Vec::with_capacity(labels.len());
        for (s, t) in labels.iter() {
            let c = cg.next();
            let mut l = codesnake::Label::new(s.range())
                .with_style(move |s| s.paint(c).whenever(cond).to_string());
            if let Some(text) = t {
                l = l.with_text(text)
            }
            colored_labels.push(l);
        }

        let Some(block) = codesnake::Block::new(lidx, colored_labels) else {
            tracing::error!("Failed to format code span, this is a bug.");
            return Ok(());
        };

        let mut prev_empty = false;
        let block = block.map_code(|s| {
            let sub = usize::from(core::mem::replace(&mut prev_empty, s.is_empty()));
            let s = s.replace('\t', "    ");
            let w = unicode_width::UnicodeWidthStr::width(&*s);
            codesnake::CodeWidth::new(s, core::cmp::max(w, 1) - sub)
        });

        writeln!(
            w,
            "{}{}{}{}",
            block.prologue(),
            "[".dim().whenever(cond),
            file_name,
            "]".dim().whenever(cond)
        )?;
        write!(w, "{block}")?;
        writeln!(w, "{}", block.epilogue())?;
    }

    let hints = err.hints();
    let mut hints = hints.iter();

    if let Some(help) = hints.next() {
        writeln!(w, "{} {}", "Help:".green().whenever(cond), help)?;
    }

    if let Some(note) = hints.next() {
        writeln!(w, "{} {}", "Note:".green().whenever(cond), note)?;
    }

    #[cfg(debug_assertions)]
    if hints.next().is_some() {
        tracing::warn!(
            hints = ?err.hints(),
            "the report builder only supports 2 hints, more will be ignored",
        );
    }
    Ok(())
}

/// Like [`Default`] but for situations where a default value does not make sense
/// and we need to recover from an error.
pub trait Recover {
    fn recover() -> Self;
}

impl<T> Recover for T
where
    T: Default,
{
    fn recover() -> Self {
        Self::default()
    }
}
