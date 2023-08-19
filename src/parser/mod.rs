//! Cooklang parser
//!
//! Grammar:
//! ```txt
//! recipe     = Newline* (line line_end)* line? Eof
//! line       = metadata | section | step
//! line_end   = soft_break | Newline+
//! soft_break = Newline !Newline
//!
//! metadata   = MetadataStart meta_key Colon meta_val
//! meta_key   = (!(Colon | Newline) ANY)*
//! meta_value = (!Newline ANY)*
//!
//! section    = Eq+ (section_name Eq*)
//! sect_name  = (!Eq ANY)*
//!
//! step       = TextStep? (component | ANY)*
//!
//! component  = c_kind modifiers? c_body note?
//! c_kind     = At | Hash | Tilde
//! c_body     = c_close | c_long | Word
//! c_long     = c_l_name c_alias? c_close
//! c_l_name   = (!(Newline | OpenBrace | Or) ANY)*
//! c_alias    = Or c_l_name
//! c_close    = OpenBrace Whitespace? Quantity? Whitespace? CloseBrace
//!
//! modifiers  = modifier+
//! modifier   = (At (OpenParen Eq? Tilde? Int CloseParen)?) | And | Plus | Minus | Question
//!
//! note       = OpenParen (!CloseParen ANY)* CloseParen
//!
//! quantity   = num_val Whitespace !(unit_sep | auto_scale | val_sep) unit
//!            | val (val_sep val)* auto_scale? (unit_sep unit)?
//!
//! unit       = (!CloseBrace ANY)*
//!
//! val_sep    = Whitespace Or Whitespace
//! auto_scale = Whitespace Star Whitespace
//! unit_sep   = Whitespace Percent Whitespace
//!
//! val        = num_val | text_val
//! text_val   = (Word | Whitespace)*
//! num_val    = mixed_num | frac | range | num
//! mixed_num  = Int Whitespace frac
//! frac       = Int Whitespace Slash Whitespace Int
//! range      = num Whitespace Minus Whitespace Num
//! num        = Float | Int
//!
//!
//! ANY        = { Any token }
//! ```
//! This is more of a guideline, there may be edge cases that this grammar does
//! not cover but the pareser does.

mod block_parser;
mod metadata;
mod quantity;
mod section;
mod step;
mod token_stream;

use std::{borrow::Cow, collections::VecDeque};

use thiserror::Error;

use crate::{
    ast::{self, Text},
    context::Context,
    error::{PassResult, RichError},
    lexer::T,
    located::Located,
    parser::{metadata::metadata_entry, section::section, step::step},
    span::Span,
    Extensions,
};

pub(crate) use block_parser::BlockParser;
use token_stream::{Token, TokenStream};

#[derive(Debug, Clone, PartialEq)]
pub enum Event<'i> {
    Metadata { key: Text<'i>, value: Text<'i> },
    Section { name: Option<Text<'i>> },
    StartStep { is_text: bool },
    EndStep { is_text: bool },
    Text(Text<'i>),
    Ingredient(Located<ast::Ingredient<'i>>),
    Cookware(Located<ast::Cookware<'i>>),
    Timer(Located<ast::Timer<'i>>),

    Error(ParserError),
    Warning(ParserWarning),
}

#[derive(Debug)]
pub(crate) struct Parser<'i, T>
where
    T: Iterator<Item = Token>,
{
    input: &'i str,
    tokens: std::iter::Peekable<T>,
    block: Vec<Token>,
    queue: VecDeque<Event<'i>>,
    extensions: Extensions,
}

impl<'input> Parser<'input, TokenStream<'input>> {
    pub fn new(input: &'input str, extensions: Extensions) -> Self {
        Self::new_from_token_iter(input, extensions, TokenStream::new(input))
    }
}

impl<'input, I> Parser<'input, I>
where
    I: Iterator<Item = Token>,
{
    pub fn new_from_token_iter(input: &'input str, extensions: Extensions, tokens: I) -> Self {
        Self {
            input,
            tokens: tokens.peekable(),
            block: Vec::new(),
            extensions,
            queue: VecDeque::new(),
        }
    }
}

fn is_empty_token(tok: &Token) -> bool {
    matches!(
        tok.kind,
        T![ws] | T![block comment] | T![line comment] | T![newline]
    )
}

fn is_line_empty(line: &[Token]) -> bool {
    line.iter().all(is_empty_token)
}

fn is_single_line_marker(first: Option<&Token>) -> bool {
    matches!(first, Some(mt![meta | =]))
}

impl<'i, I> Parser<'i, I>
where
    I: Iterator<Item = Token>,
{
    fn pull_line(&mut self) -> Option<&[Token]> {
        let last_line_end = self.block.len();
        for tok in self.tokens.by_ref() {
            self.block.push(tok);
            if tok.kind == T![newline] {
                break;
            }
        }
        let line = &self.block[last_line_end..];
        (!line.is_empty()).then_some(line)
    }

    /// Advances a block. Store the tokens, newline/eof excluded.
    pub(crate) fn next_block(&mut self) -> Option<()> {
        self.block.clear();
        let multiline_ext = self.extensions.contains(Extensions::MULTILINE_STEPS);

        // start and end are used to track the "non empty" part of the block
        let mut start = 0;
        let mut end;

        let mut current_line = self.pull_line()?;

        // Eat empty lines
        while is_line_empty(current_line) {
            start = self.block.len();
            current_line = self.pull_line()?;
        }

        // Check if more lines have to be consumed
        let multiline = multiline_ext && !is_single_line_marker(current_line.first());
        end = self.block.len();
        if multiline {
            loop {
                if is_single_line_marker(self.tokens.peek()) {
                    break;
                }
                match self.pull_line() {
                    None => break,
                    Some(line) if is_line_empty(line) => break,
                    _ => {}
                }
                end = self.block.len();
            }
        }

        // trim trailing newline
        while let mt![newline] = self.block[end - 1] {
            if end <= start {
                break;
            }
            end -= 1;
        }
        // trim empty lines
        let trimmed_block = &self.block[start..end];
        if trimmed_block.is_empty() {
            return None;
        }

        let mut bp = BlockParser::new(trimmed_block, self.input, &mut self.queue, self.extensions);
        parse_block(&mut bp);
        bp.finish();

        Some(())
    }

    fn next_metadata_block(&mut self) -> Option<()> {
        self.block.clear();

        let mut last = T![newline];
        let mut in_meta = false;

        for tok in self.tokens.by_ref() {
            if in_meta {
                if tok.kind == T![newline] {
                    break;
                } else {
                    self.block.push(tok);
                }
            } else if tok.kind == T![meta] && last == T![newline] {
                self.block.push(tok);
                in_meta = true;
            }
            last = tok.kind;
        }

        if self.block.is_empty() {
            return None;
        }

        let mut bp = BlockParser::new(&self.block, self.input, &mut self.queue, self.extensions);
        if let Some(ev) = metadata_entry(&mut bp) {
            bp.event(ev);
        }
        bp.finish();

        Some(())
    }

    pub(crate) fn next_metadata(&mut self) -> Option<Event<'i>> {
        self.next_metadata_block()?;
        self.queue.pop_front()
    }
}

impl<'i, T> Iterator for Parser<'i, T>
where
    T: Iterator<Item = Token>,
{
    type Item = Event<'i>;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.pop_front().or_else(|| {
            self.next_block()?;
            self.next()
        })
    }
}

fn parse_block(line: &mut BlockParser) {
    let meta_or_section = match line.peek() {
        T![meta] => line.with_recover(metadata_entry),
        T![=] => line.with_recover(section),
        _ => None,
    };

    if let Some(ev) = meta_or_section {
        line.event(ev);
        return;
    }
    step(line);
}

/// Parse a recipe into an [`Ast`](ast::Ast)
#[tracing::instrument(level = "debug", skip_all, fields(len = input.len()))]
pub fn parse<'input>(
    input: &'input str,
    extensions: Extensions,
) -> PassResult<ast::Ast<'input>, ParserError, ParserWarning> {
    let mut parser = Parser::new(input, extensions);
    let mut blocks = Vec::new();
    let mut items = Vec::new();
    let mut ctx = Context::default();
    for event in parser.by_ref() {
        match event {
            Event::Metadata { key, value } => blocks.push(ast::Block::Metadata { key, value }),
            Event::Section { name } => blocks.push(ast::Block::Section { name }),
            Event::StartStep { .. } => items.clear(),
            Event::EndStep { is_text } => {
                if !items.is_empty() {
                    blocks.push(ast::Block::Step {
                        is_text,
                        items: std::mem::take(&mut items),
                    })
                }
            }
            Event::Text(t) => items.push(ast::Item::Text(t)),
            Event::Ingredient(c) => items.push(ast::Item::Ingredient(c)),
            Event::Cookware(c) => items.push(ast::Item::Cookware(c)),
            Event::Timer(c) => items.push(ast::Item::Timer(c)),
            Event::Error(e) => ctx.error(e),
            Event::Warning(w) => ctx.warn(w),
        }
    }
    let ast = ast::Ast { blocks };
    ctx.finish(Some(ast))
}

/// Parse only the recipe metadata into an [`Ast`](ast::Ast).
///
/// This will skip every line that is not metadata. Is faster than [`parse`].
#[tracing::instrument(level = "debug", skip_all, fields(len = input.len()))]
pub fn parse_metadata<'input>(
    input: &'input str,
) -> PassResult<ast::Ast<'input>, ParserError, ParserWarning> {
    let mut parser = Parser::new(input, Extensions::empty());
    let mut blocks = Vec::new();
    let mut ctx = Context::default();
    while let Some(ev) = parser.next_metadata() {
        match ev {
            Event::Metadata { key, value } => blocks.push(ast::Block::Metadata { key, value }),
            Event::Error(e) => ctx.error(e),
            Event::Warning(w) => ctx.warn(w),
            _ => {}
        }
    }
    let ast = ast::Ast { blocks };
    ctx.finish(Some(ast))
}

/// get the span for a slice of tokens. panics if the slice is empty
pub(crate) fn tokens_span(tokens: &[Token]) -> Span {
    debug_assert!(!tokens.is_empty(), "tokens_span tokens empty");
    let start = tokens.first().unwrap().span.start();
    let end = tokens.last().unwrap().span.end();
    Span::new(start, end)
}

// match token type
macro_rules! mt {
    ($($reprs:tt)|*) => {
        $(Token {
            kind: T![$reprs],
            ..
        })|+
    }
}
pub(crate) use mt;

/// Errors generated by [`parse`] and [`parse_metadata`].
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ParserError {
    #[error("A {container} is missing: {what}")]
    ComponentPartMissing {
        container: &'static str,
        what: &'static str,
        expected_pos: Span,
    },

    #[error("A {container} cannot have: {what}")]
    ComponentPartNotAllowed {
        container: &'static str,
        what: &'static str,
        to_remove: Span,
        help: Option<&'static str>,
    },

    #[error("Invalid {container} {what}: {reason}")]
    ComponentPartInvalid {
        container: &'static str,
        what: &'static str,
        reason: &'static str,
        labels: Vec<(Span, Option<Cow<'static, str>>)>,
        help: Option<&'static str>,
    },

    #[error("Duplicate ingredient modifier: {dup}")]
    DuplicateModifiers { modifiers_span: Span, dup: String },

    #[error("Error parsing integer number")]
    ParseInt {
        bad_bit: Span,
        source: std::num::ParseIntError,
    },

    #[error("Error parsing decimal number")]
    ParseFloat {
        bad_bit: Span,
        source: std::num::ParseFloatError,
    },

    #[error("Division by zero")]
    DivisionByZero { bad_bit: Span },

    #[error("Quantity scaling conflict")]
    QuantityScalingConflict { bad_bit: Span },
}

/// Warnings generated by [`parse`] and [`parse_metadata`].
#[derive(Debug, Error, Clone, PartialEq)]
pub enum ParserWarning {
    #[error("Empty metadata value for key: {key}")]
    EmptyMetadataValue { key: Located<String> },
    #[error("A {container} cannot have {what}, it will be ignored")]
    ComponentPartIgnored {
        container: &'static str,
        what: &'static str,
        ignored: Span,
        help: Option<&'static str>,
    },
}

impl RichError for ParserError {
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
        use crate::error::label;
        match self {
            ParserError::ComponentPartMissing {
                expected_pos: component_span,
                what,
                ..
            } => {
                vec![label!(component_span, format!("expected {what}"))]
            }
            ParserError::ComponentPartNotAllowed { to_remove, .. } => {
                vec![label!(to_remove, "remove this")]
            }
            ParserError::ComponentPartInvalid { labels, .. } => labels.clone(),
            ParserError::DuplicateModifiers { modifiers_span, .. } => vec![label!(modifiers_span)],
            ParserError::ParseInt { bad_bit, .. } => vec![label!(bad_bit)],
            ParserError::ParseFloat { bad_bit, .. } => vec![label!(bad_bit)],
            ParserError::DivisionByZero { bad_bit } => vec![label!(bad_bit)],
            ParserError::QuantityScalingConflict { bad_bit } => vec![label!(bad_bit)],
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        use crate::error::help;
        match self {
            ParserError::ComponentPartNotAllowed { help, .. } => help!(opt help),
            ParserError::ComponentPartInvalid { help, .. } => help!(opt help),
            ParserError::DuplicateModifiers { .. } => help!("Remove duplicate modifiers"),
            ParserError::DivisionByZero { .. } => {
                help!("Change this please, we don't want an infinite amount of anything")
            }
            ParserError::QuantityScalingConflict { .. } => help!("A quantity cannot have the auto scaling marker (*) and have fixed values at the same time"),
            _ => None,
        }
    }

    fn code(&self) -> Option<&'static str> {
        Some("parser")
    }
}

impl RichError for ParserWarning {
    fn labels(&self) -> Vec<(Span, Option<Cow<'static, str>>)> {
        use crate::error::label;
        match self {
            ParserWarning::EmptyMetadataValue { key } => {
                vec![label!(key)]
            }
            ParserWarning::ComponentPartIgnored { ignored, .. } => {
                vec![label!(ignored, "this is ignored")]
            }
        }
    }

    fn help(&self) -> Option<Cow<'static, str>> {
        use crate::error::help;
        match self {
            ParserWarning::EmptyMetadataValue { .. } => None,
            ParserWarning::ComponentPartIgnored { help, .. } => help!(opt help),
        }
    }

    fn code(&self) -> Option<&'static str> {
        Some("parser")
    }

    fn kind(&self) -> ariadne::ReportKind {
        ariadne::ReportKind::Warning
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn just_metadata() {
        let (ast, warn, err) = parse_metadata(
            r#">> entry: true
a test @step @salt{1%mg} more text
a test @step @salt{1%mg} more text
a test @step @salt{1%mg} more text
>> entry2: uwu
a test @step @salt{1%mg} more text
"#,
        )
        .into_tuple();
        assert!(warn.is_empty());
        assert!(err.is_empty());
        assert_eq!(
            ast.unwrap().blocks,
            vec![
                Block::Metadata {
                    key: Text::from_str(" entry", 2),
                    value: Text::from_str(" true", 10)
                },
                Block::Metadata {
                    key: Text::from_str(" entry2", 126),
                    value: Text::from_str(" uwu", 134)
                },
            ]
        );
    }

    #[test]
    fn multiline_spaces() {
        let (ast, warn, err) = parse(
            "  This is a step           -- comment\n and this line continues  -- another comment",
            Extensions::MULTILINE_STEPS,
        )
        .into_tuple();

        // Only whitespace between line should be trimmed
        assert!(warn.is_empty());
        assert!(err.is_empty());
        assert_eq!(
            ast.unwrap().blocks,
            vec![Block::Step {
                is_text: false,
                items: vec![Item::Text({
                    let mut t = Text::empty(0);
                    t.append_str("  This is a step           ", 0);
                    t.append_fragment(TextFragment::soft_break("\n", 37));
                    t.append_str(" and this line continues  ", 39);
                    t
                })]
            }]
        );
    }
}
