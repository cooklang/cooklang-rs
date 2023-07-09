//! Error type, formatting and utilities.

use std::borrow::Cow;
use std::ops::Deref;

use thiserror::Error;

/// Errors and warnings container with fancy formatting
///
/// The [`Display`](std::fmt::Display) implementation is not fancy formatting,
/// use one of the print or write methods.
#[derive(Debug, Clone)]
pub struct Report<E, W> {
    pub(crate) errors: Vec<E>,
    pub(crate) warnings: Vec<W>,
}

pub type CooklangReport = Report<CooklangError, CooklangWarning>;

impl<E, W> Report<E, W> {
    /// Create a new report
    pub fn new(errors: Vec<E>, warnings: Vec<W>) -> Self {
        Self { errors, warnings }
    }

    /// Errors of the report
    pub fn errors(&self) -> &[E] {
        &self.errors
    }

    /// Warnings of the report
    pub fn warnings(&self) -> &[W] {
        &self.warnings
    }

    /// Check if the reports has errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    /// Check if the reports has warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
    /// Check if the reports doesn't have errors or warnings
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty()
    }
}

impl<E, W> Report<E, W>
where
    E: RichError,
    W: RichError,
{
    /// Write a formatted report
    pub fn write(
        &self,
        file_name: &str,
        source_code: &str,
        hide_warnings: bool,
        color: bool,
        w: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        let mut cache = DummyCache::new(file_name, source_code);
        if !hide_warnings {
            for warn in &self.warnings {
                build_report(warn, file_name, source_code, color).write(&mut cache, &mut *w)?;
            }
        }
        for err in &self.errors {
            build_report(err, file_name, source_code, color).write(&mut cache, &mut *w)?;
        }
        Ok(())
    }

    /// Prints a formatted report to stdout
    pub fn print(
        &self,
        file_name: &str,
        source_code: &str,
        hide_warnings: bool,
        color: bool,
    ) -> std::io::Result<()> {
        self.write(
            file_name,
            source_code,
            hide_warnings,
            color,
            &mut std::io::stdout(),
        )
    }

    /// Prints a formatted report to stderr
    pub fn eprint(
        &self,
        file_name: &str,
        source_code: &str,
        hide_warnings: bool,
        color: bool,
    ) -> std::io::Result<()> {
        self.write(
            file_name,
            source_code,
            hide_warnings,
            color,
            &mut std::io::stderr(),
        )
    }
}

impl<E, W> std::fmt::Display for Report<E, W>
where
    E: std::fmt::Display,
    W: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let errors = &self.errors;
        let warnings = &self.warnings;
        if errors.len() == 1 {
            errors[0].fmt(f)?;
        } else if warnings.len() == 1 {
            warnings[0].fmt(f)?;
        } else {
            match (errors.is_empty(), warnings.is_empty()) {
                (true, true) => writeln!(f, "Unknown error")?,
                (true, false) => writeln!(f, "Multiple warnings:")?,
                (false, _) => writeln!(f, "Multiple errors:")?,
            }
            for warn in warnings {
                warn.fmt(f)?;
            }
            for err in errors {
                err.fmt(f)?;
            }
        }
        Ok(())
    }
}
impl<E, W> std::error::Error for Report<E, W>
where
    E: std::fmt::Display + std::fmt::Debug,
    W: std::fmt::Display + std::fmt::Debug,
{
}

/// Partial [`Report`] only for warnings
pub struct Warnings<W>(Vec<W>);

impl<W> Warnings<W> {
    pub fn new(warnings: Vec<W>) -> Self {
        Self(warnings)
    }

    pub fn into_report<E>(self) -> Report<E, W> {
        self.into()
    }
}

impl<W: RichError> Warnings<W> {
    /// Write a formatted report
    pub fn write(
        &self,
        file_name: &str,
        source_code: &str,
        color: bool,
        w: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        let mut cache = DummyCache::new(file_name, source_code);
        for warn in &self.0 {
            build_report(warn, file_name, source_code, color).write(&mut cache, &mut *w)?;
        }
        Ok(())
    }

    /// Prints a formatted report to stdout
    pub fn print(&self, file_name: &str, source_code: &str, color: bool) -> std::io::Result<()> {
        self.write(file_name, source_code, color, &mut std::io::stdout())
    }

    /// Prints a formatted report to stderr
    pub fn eprint(&self, file_name: &str, source_code: &str, color: bool) -> std::io::Result<()> {
        self.write(file_name, source_code, color, &mut std::io::stderr())
    }
}

impl<E, W> From<Warnings<W>> for Report<E, W> {
    fn from(value: Warnings<W>) -> Self {
        Self::new(vec![], value.0)
    }
}

impl<W> Deref for Warnings<W> {
    type Target = [W];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Output from the different passes of the parsing process
#[derive(Debug)]
pub struct PassResult<T, E, W> {
    output: Option<T>,
    warnings: Vec<W>,
    errors: Vec<E>,
}

impl<T, E, W> PassResult<T, E, W> {
    pub(crate) fn new(output: Option<T>, warnings: Vec<W>, errors: Vec<E>) -> Self {
        Self {
            output,
            warnings,
            errors,
        }
    }

    /// Check if the result has any output. It may not be valid.
    pub fn has_output(&self) -> bool {
        self.output.is_some()
    }

    /// Check if the result has errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if the result has warnings.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Check if the result is invalid.
    ///
    /// Output, if any, should be discarded or used knowing that it contains
    /// errors or is incomplete.
    pub fn invalid(&self) -> bool {
        self.has_errors() || !self.has_output()
    }

    /// Get the output
    pub fn output(&self) -> Option<&T> {
        self.output.as_ref()
    }

    /// Get the warnings
    pub fn warnings(&self) -> &[W] {
        &self.warnings
    }

    /// Get the errors
    pub fn errors(&self) -> &[E] {
        &self.errors
    }

    /// Transform into a common rust [`Result`]
    pub fn into_result(mut self) -> Result<(T, Warnings<W>), Report<E, W>> {
        if let Some(o) = self.output.take() {
            if self.errors.is_empty() {
                return Ok((o, Warnings::new(self.warnings)));
            }
        }
        Err(self.into_report())
    }

    /// Transform into a [`Report`] discarding the output
    pub fn into_report(self) -> Report<E, W> {
        Report {
            errors: self.errors,
            warnings: self.warnings,
        }
    }

    /// Take the output discarding the errors/warnings
    pub fn take_output(&mut self) -> Option<T> {
        self.output.take()
    }

    /// Transform into the ouput discarding errors/warnings
    pub fn into_output(self) -> Option<T> {
        self.output
    }

    /// Transform into errors discarding output and warnings
    pub fn into_errors(self) -> Vec<E> {
        self.errors
    }

    /// Transform into warnings discarding output and errors
    pub fn into_warnings(self) -> Vec<W> {
        self.warnings
    }

    /// Get output, errors and warnings in a tuple
    pub fn into_tuple(self) -> (Option<T>, Vec<W>, Vec<E>) {
        (self.output, self.warnings, self.errors)
    }

    pub(crate) fn into_context_result<E2, W2>(self) -> PassResult<T, E2, W2>
    where
        E2: From<E>,
        W2: From<W>,
    {
        PassResult {
            output: self.output,
            errors: self.errors.into_iter().map(Into::into).collect(),
            warnings: self.warnings.into_iter().map(Into::into).collect(),
        }
    }

    pub(crate) fn discard_output<T2>(self) -> PassResult<T2, E, W> {
        PassResult {
            output: None,
            warnings: self.warnings,
            errors: self.errors,
        }
    }

    pub(crate) fn merge<T2>(mut self, mut other: PassResult<T2, E, W>) -> Self {
        other.errors.append(&mut self.errors);
        other.warnings.append(&mut self.warnings);
        self.errors = other.errors;
        self.warnings = other.warnings;
        self
    }

    /// Map the inner output
    pub fn map<F, O>(self, f: F) -> PassResult<O, E, W>
    where
        F: FnOnce(T) -> O,
    {
        PassResult {
            output: self.output.map(f),
            warnings: self.warnings,
            errors: self.errors,
        }
    }

    /// Map the inner output with a fallible function
    pub fn try_map<F, O, E2>(self, f: F) -> Result<PassResult<O, E, W>, E2>
    where
        F: FnOnce(T) -> Result<O, E2>,
    {
        let output = self.output.map(f).transpose()?;
        Ok(PassResult {
            output,
            warnings: self.warnings,
            errors: self.errors,
        })
    }
}

/// Trait to enhace errors with rich metadata
pub trait RichError: std::error::Error {
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
        vec![]
    }
    fn help(&self) -> Option<Cow<'static, str>> {
        None
    }
    fn note(&self) -> Option<Cow<'static, str>> {
        None
    }
    fn code(&self) -> Option<&'static str> {
        None
    }
    fn kind(&self) -> ariadne::ReportKind {
        ariadne::ReportKind::Error
    }
}

macro_rules! label {
    ($span:expr) => {
        ($span.to_owned().into(), None)
    };
    ($span:expr, $message:expr) => {
        ($span.to_owned().into(), Some($message.into()))
    };
}
pub(crate) use label;

macro_rules! help {
    () => {
        None
    };
    ($help:expr) => {
        Some($help.into())
    };
    (opt $help:expr) => {
        $help.map(|h| h.into())
    };
}
pub(crate) use help;
pub(crate) use help as note;

use crate::span::Span;

/// Writes a rich error report
///
/// This function should not be used in a loop as each call will
/// perform a light parse of the whole source code. To print many rich errors
/// use [`Report`].
pub fn write_rich_error(
    error: &dyn RichError,
    file_name: &str,
    source_code: &str,
    color: bool,
    w: impl std::io::Write,
) -> std::io::Result<()> {
    let mut cache = DummyCache::new(file_name, source_code);
    let report = build_report(error, file_name, source_code, color);
    report.write(&mut cache, w)
}

fn build_report<'a>(
    err: &'a dyn RichError,
    file_name: &str,
    src_code: &str,
    color: bool,
) -> ariadne::Report<'a> {
    use ariadne::{Color, ColorGenerator, Fmt, Label, Report};

    let labels = err
        .labels()
        .into_iter()
        .map(|(s, t)| (s.to_chars_span(src_code, file_name).range(), t))
        .collect::<Vec<_>>();

    // The start of the first span
    let offset = labels.iter().map(|l| l.0.start).min().unwrap_or_default();

    let mut r = Report::build(err.kind(), (), offset)
        .with_config(ariadne::Config::default().with_color(color));

    if let Some(source) = err.source() {
        let color = match err.kind() {
            ariadne::ReportKind::Error => Color::Red,
            ariadne::ReportKind::Warning => Color::Yellow,
            ariadne::ReportKind::Advice => Color::Fixed(147),
            ariadne::ReportKind::Custom(_, c) => c,
        };
        let message = format!("{err}\n  {} {source}", "╰▶ ".fg(color));
        r.set_message(message);
    } else {
        r.set_message(err);
    }

    let mut c = ColorGenerator::new();
    r.add_labels(labels.into_iter().enumerate().map(|(order, (span, text))| {
        let mut l = Label::new(span)
            .with_order(order as i32)
            .with_color(c.next());
        if let Some(text) = text {
            l = l.with_message(text);
        }
        l
    }));

    if let Some(help) = err.help() {
        r.set_help(help);
    }

    if let Some(note) = err.note() {
        r.set_note(note);
    }

    r.finish()
}

// This is a ariadne cache that only supports one file.
// If needed it can be expanded to a full cache as the source id is already
// stored in CharsSpan (the ariadne::Span)
struct DummyCache(String, ariadne::Source);
impl DummyCache {
    fn new(file_name: &str, src_code: &str) -> Self {
        Self(file_name.into(), src_code.into())
    }
}
impl ariadne::Cache<()> for DummyCache {
    fn fetch(&mut self, _id: &()) -> Result<&ariadne::Source, Box<dyn std::fmt::Debug + '_>> {
        Ok(&self.1)
    }

    fn display<'a>(&self, _id: &'a ()) -> Option<Box<dyn std::fmt::Display + 'a>> {
        Some(Box::new(self.0.clone()))
    }
}

/// General cooklang error type
#[derive(Debug, Error)]
pub enum CooklangError {
    /// An error in the parse pass
    #[error(transparent)]
    Parser(#[from] crate::parser::ParserError),
    /// An error in the analysis pass
    #[error(transparent)]
    Analysis(#[from] crate::analysis::AnalysisError),
    /// Error related to I/O
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// General cooklang warning type
#[derive(Debug, Error)]
#[error(transparent)]
pub enum CooklangWarning {
    /// A waring in the parse pass
    Parser(#[from] crate::parser::ParserWarning),
    /// A waring in the analysis pass
    Analysis(#[from] crate::analysis::AnalysisWarning),
}

impl RichError for CooklangError {
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
        match self {
            CooklangError::Parser(e) => e.labels(),
            CooklangError::Analysis(e) => e.labels(),
            CooklangError::Io(_) => vec![],
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        match self {
            CooklangError::Parser(e) => e.help(),
            CooklangError::Analysis(e) => e.help(),
            CooklangError::Io(_) => None,
        }
    }

    fn note(&self) -> Option<Cow<'static, str>> {
        match self {
            CooklangError::Parser(e) => e.note(),
            CooklangError::Analysis(e) => e.note(),
            CooklangError::Io(_) => None,
        }
    }

    fn code(&self) -> Option<&'static str> {
        match self {
            CooklangError::Parser(e) => e.code(),
            CooklangError::Analysis(e) => e.code(),
            CooklangError::Io(_) => Some("io"),
        }
    }
}

impl RichError for CooklangWarning {
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
        match self {
            CooklangWarning::Parser(e) => e.labels(),
            CooklangWarning::Analysis(e) => e.labels(),
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        match self {
            CooklangWarning::Parser(e) => e.help(),
            CooklangWarning::Analysis(e) => e.help(),
        }
    }

    fn code(&self) -> Option<&'static str> {
        match self {
            CooklangWarning::Parser(e) => e.code(),
            CooklangWarning::Analysis(e) => e.code(),
        }
    }

    fn kind(&self) -> ariadne::ReportKind {
        ariadne::ReportKind::Warning
    }
}
