mod cursor;

pub use cursor::Cursor;

use finl_unicode::categories::CharacterCategories;

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub len: u32,
}

impl Token {
    fn new(kind: TokenKind, len: u32) -> Token {
        Token { kind, len }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    /// ">>"
    MetadataStart,
    /// ">"
    TextStep,
    /// ":"
    Colon,
    /// "@"
    At,
    /// "#"
    Hash,
    /// "~"
    Tilde,
    /// "?"
    Question,
    /// "+"
    Plus,
    /// "-"
    Minus,
    /// "/"
    Slash,
    /// "*"
    Star,
    /// "&"
    And,
    /// "|"
    Or,
    /// "="
    Eq,
    /// "%"
    Percent,
    /// "{"
    OpenBrace,
    /// "}"
    CloseBrace,
    /// "("
    OpenParen,
    /// ")"
    CloseParen,
    /// "."
    Dot,

    /// "14", "0", but not "014"
    Int,
    /// "014", but not "14"
    ZeroInt,
    /// any other unicode punctuation characters
    Punctuation,
    /// Everything else, a "\" escapes the next char
    Word,
    /// "\" followed by any char
    Escaped,

    /// " " and \t
    Whitespace,
    /// \r\n and \n
    Newline,
    /// "-- any until newline"
    LineComment,
    /// "[- any until EOF or close -]"
    BlockComment,

    /// End of input
    Eof,
}

fn is_whitespace(c: char) -> bool {
    c.is_separator_space() || c == '\t'
}

fn is_word_char(c: char) -> bool {
    // this is critical code for performance, check lexer benchmark before and after chaging it
    match c {
        c if c.is_alphabetic() => true, // quick return true
        ' ' | '\n' | '\r' | '\t' | '0'..='9' | '.' => false, // common chars that break a word
        '>' | ':' | '@' | '#' | '~' | '?' | '+' | '-' | '/' | '*' | '&' | '|' | '=' | '%' | '{'
        | '}' | '(' | ')' => false,
        c if c.is_separator_space() || c.is_punctuation() => false, // '\' (escape) is punctuation and not common, so I will leave it here
        _ => true,
    }
}

impl Cursor<'_> {
    pub fn advance_token(&mut self) -> Token {
        let current = match self.bump() {
            Some(c) => c,
            None => return Token::new(TokenKind::Eof, 0),
        };

        let token_kind = match current {
            '\\' => {
                self.bump(); // any
                TokenKind::Escaped
            }

            '>' => {
                if self.first() == '>' {
                    self.bump(); // '>'
                    TokenKind::MetadataStart
                } else {
                    TokenKind::TextStep
                }
            }
            '-' => match self.first() {
                '-' => self.line_comment(),
                _ => TokenKind::Minus,
            },
            '[' if self.first() == '-' => self.block_comment(),
            '\n' => TokenKind::Newline,
            '\r' if self.first() == '\n' => {
                self.bump(); // '\n'
                TokenKind::Newline
            }
            c @ '0'..='9' => self.number(c),

            ':' => TokenKind::Colon,
            '@' => TokenKind::At,
            '#' => TokenKind::Hash,
            '~' => TokenKind::Tilde,
            '?' => TokenKind::Question,
            '+' => TokenKind::Plus,
            '/' => TokenKind::Slash,
            '*' => TokenKind::Star,
            '&' => TokenKind::And,
            '|' => TokenKind::Or,
            '%' => TokenKind::Percent,
            '=' => TokenKind::Eq,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '.' => TokenKind::Dot,

            c if is_whitespace(c) => self.whitespace(),
            c if c.is_punctuation() => TokenKind::Punctuation,

            // anything else, word
            _ => self.word(),
        };
        let token = Token::new(token_kind, self.pos_within_token());
        self.reset_pos_within_token();
        token
    }

    fn line_comment(&mut self) -> TokenKind {
        debug_assert!(self.prev() == '-' && self.first() == '-');
        // this makes the next newline don't have the '\r' if on windows, but
        // I don't think that's a problem
        self.eat_while(|c| c != '\n');
        TokenKind::LineComment
    }

    fn block_comment(&mut self) -> TokenKind {
        debug_assert!(self.prev() == '[' && self.first() == '-');
        self.bump(); // '-'
        while let Some(c) = self.bump() {
            match c {
                '-' if self.first() == ']' => {
                    self.bump();
                    break;
                }
                _ => {}
            }
        }
        TokenKind::BlockComment
    }

    fn word(&mut self) -> TokenKind {
        debug_assert!(self.pos_within_token() > 0); // at least one char
        self.eat_while(is_word_char);
        TokenKind::Word
    }

    fn whitespace(&mut self) -> TokenKind {
        debug_assert!(is_whitespace(self.prev()));
        self.eat_while(is_whitespace);
        TokenKind::Whitespace
    }

    fn number(&mut self, c: char) -> TokenKind {
        debug_assert!(self.prev().is_ascii_digit());
        self.eat_while(|c| c.is_ascii_digit());
        let leading_zero = c == '0' && self.pos_within_token() > 1;
        if leading_zero {
            TokenKind::ZeroInt
        } else {
            TokenKind::Int
        }
    }
}

/// Shorthand macro for [`TokenKind`]
macro_rules! T {
    [+] => {
        $crate::lexer::TokenKind::Plus
    };
    [@] => {
        $crate::lexer::TokenKind::At
    };
    [#] => {
        $crate::lexer::TokenKind::Hash
    };
    [~] => {
        $crate::lexer::TokenKind::Tilde
    };
    [?] => {
        $crate::lexer::TokenKind::Question
    };
    [-] => {
        $crate::lexer::TokenKind::Minus
    };
    [*] => {
        $crate::lexer::TokenKind::Star
    };
    [%] => {
        $crate::lexer::TokenKind::Percent
    };
    [/] => {
        $crate::lexer::TokenKind::Slash
    };
    [=] => {
        $crate::lexer::TokenKind::Eq
    };
    [&] => {
        $crate::lexer::TokenKind::And
    };
    [|] => {
        $crate::lexer::TokenKind::Or
    };
    [:] => {
        $crate::lexer::TokenKind::Colon
    };
    [>] => {
        $crate::lexer::TokenKind::TextStep
    };
    ['{'] => {
        $crate::lexer::TokenKind::OpenBrace
    };
    ['}'] => {
        $crate::lexer::TokenKind::CloseBrace
    };
    ['('] => {
        $crate::lexer::TokenKind::OpenParen
    };
    [')'] => {
        $crate::lexer::TokenKind::CloseParen
    };
    [.] => {
        $crate::lexer::TokenKind::Dot
    };
    [word] => {
        $crate::lexer::TokenKind::Word
    };
    [escaped] => {
        $crate::lexer::TokenKind::Escaped
    };
    [line comment] => {
        $crate::lexer::TokenKind::LineComment
    };
    [block comment] => {
        $crate::lexer::TokenKind::BlockComment
    };
    [int] => {
        $crate::lexer::TokenKind::Int
    };
    [zeroint] => {
        $crate::lexer::TokenKind::ZeroInt
    };
    [meta] => {
        $crate::lexer::TokenKind::MetadataStart
    };
    [>>] => {
        $crate::lexer::TokenKind::MetadataStart
    };
    [ws] => {
        $crate::lexer::TokenKind::Whitespace
    };
    [punctuation] => {
        $crate::lexer::TokenKind::Punctuation
    };
    [newline] => {
        $crate::lexer::TokenKind::Newline
    };
    [eof] => {
        $crate::lexer::TokenKind::Eof
    };
}
pub(crate) use T;

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind::*;

    fn tokenize(input: &str) -> impl Iterator<Item = Token> + '_ {
        let mut cursor = Cursor::new(input);
        std::iter::from_fn(move || {
            let token = cursor.advance_token();
            if token.kind != TokenKind::Eof {
                Some(token)
            } else {
                None
            }
        })
    }

    macro_rules! t {
        ($input:expr, $token_kinds:expr) => {
            let got: Vec<TokenKind> = tokenize($input).map(|t| t.kind).collect();
            assert_eq!(got, $token_kinds, "Input was: '{}'", $input)
        };
    }

    #[test]
    fn word() {
        t!("basic", vec![Word]);
        t!("word.word", vec![Word, Dot, Word]);
        t!("word⸫word", vec![Word, Punctuation, Word]);
        t!("👀", vec![Word]);
        t!("👀more", vec![Word]);
        t!("thing👀more", vec![Word]);

        t!("word3,", vec![Word, Int, Punctuation]);
        t!("word03.", vec![Word, ZeroInt, Dot]);
        t!("phraseends.3", vec![Word, Dot, Int]);
        t!("phraseends. 3", vec![Word, Dot, Whitespace, Int]);
        t!("phraseends .3", vec![Word, Whitespace, Dot, Int]);

        t!("two words", vec![Word, Whitespace, Word]);
        t!("two words", vec![Word, Whitespace, Word]); // unicode whitespace U+2009
        t!("word\nanother", vec![Word, Newline, Word]);

        // composed emojis more than one char
        t!("👩🏿‍🔬", vec![Word]);
        t!("\u{1F1EA}\u{1F1F8}", vec![Word]);
        t!("thing👩🏿‍🔬more", vec![Word]);
        t!("thing👩🏿‍🔬more", vec![Word]);
    }

    #[test]
    fn number() {
        t!("1", vec![Int]);
        t!("0", vec![Int]);
        t!("01", vec![ZeroInt]);
        t!("01.3", vec![ZeroInt, Dot, Int]);
        t!("1.3", vec![Int, Dot, Int]);
        t!(".3", vec![Dot, Int]);
        t!("0.3", vec![Int, Dot, Int]);
        t!("0.03", vec![Int, Dot, ZeroInt]);
        t!("{.3}", vec![OpenBrace, Dot, Int, CloseBrace]);
        t!("14.", vec![Int, Dot]);
    }

    #[test]
    fn comment() {
        t!("-- a line comment", vec![LineComment]);
        t!("[- a block comment -]", vec![BlockComment]);
        t!(
            "a word [- then comment -] the more",
            vec![
                Word,
                Whitespace,
                Word,
                Whitespace,
                BlockComment,
                Whitespace,
                Word,
                Whitespace,
                Word
            ]
        );
        t!(
            "word -- and line comment",
            vec![Word, Whitespace, LineComment]
        );
        t!(
            "word -- and line comment\nmore",
            vec![Word, Whitespace, LineComment, Newline, Word]
        );
        t!(
            "word [- non closed block\ncomment",
            vec![Word, Whitespace, BlockComment]
        );
    }

    #[test]
    fn test_component() {
        t!("@basic", vec![At, Word]);
        t!("#basic", vec![Hash, Word]);
        t!("~basic", vec![Tilde, Word]);
        t!("@single word", vec![At, Word, Whitespace, Word]);
        t!(
            "@multi word{}",
            vec![At, Word, Whitespace, Word, OpenBrace, CloseBrace]
        );
        t!("@qty{3}", vec![At, Word, OpenBrace, Int, CloseBrace]);
        t!(
            "@qty{3}(note)",
            vec![At, Word, OpenBrace, Int, CloseBrace, OpenParen, Word, CloseParen]
        );
    }

    #[test]
    fn recipe() {
        const S: TokenKind = TokenKind::Whitespace;
        const L: TokenKind = TokenKind::Newline;
        let input = r#"
>> key: value
Just let him cook.

Use @sauce{100%ml} and @love.
"#;
        #[rustfmt::skip]
        t!(input, vec![
            L,
            MetadataStart, S, Word, Colon, S, Word, L,
            Word, S, Word, S, Word, S, Word, Dot, L,
            L,
            Word, S, At, Word, OpenBrace, Int, Percent, Word, CloseBrace, S, Word, S, At, Word, Dot, L
        ]);
    }
}
