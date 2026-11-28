//! Inline formatting in notes (spec proposal 0022)
//!
//! Emphasis, strong emphasis and inline links, read with the rules of
//! `CommonMark` 0.31.2 (sections 6.2 and 6.3 and the delimiter algorithm in its
//! appendix). Nothing else of Markdown is read.

//!
//! The input is the text of the note as a list of characters, each marked if
//! it was escaped with a backslash. An escaped character is always text: it
//! never opens or closes emphasis or a link.
//!
//! The text is split into nodes: runs of plain text, and the text of each
//! delimiter run (`*`, `**`, `_`...) or `[`. Matching delimiters and links
//! records ranges of nodes with a format, and removes the delimiter characters
//! from their nodes. As `CommonMark` gives a tree, these ranges nest, and the
//! formatting of each node is the union of the ranges that contain it, which
//! is all that flat [`TextRun`]s need.

use finl_unicode::categories::CharacterCategories;

use crate::model::TextRun;

/// A character of a note, after comments are removed and lines are joined
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NoteChar {
    pub c: char,
    /// Escaped with a backslash: never opens or closes formatting
    pub escaped: bool,
}

/// Read the formatting of a note into runs
pub(crate) fn parse_note(chars: &[NoteChar]) -> Vec<TextRun> {
    let mut p = InlineParser {
        input: chars,
        pos: 0,
        nodes: Vec::new(),
        text_open: false,
        spans: Vec::new(),
        links: Vec::new(),
        delims: Vec::new(),
        last_delim: None,
        brackets: Vec::new(),
    };
    p.parse();
    p.into_runs()
}

/// Same limit as `CommonMark` implementations for parentheses in a link
/// destination
const MAX_NESTED_PARENS: usize = 32;

#[derive(Debug, Clone, Copy)]
enum Format {
    Strong,
    Emphasis,
}

/// An entry of the delimiter list for `*` and `_`
///
/// Entries are pushed in the order of the text, so their index in
/// [`InlineParser::delims`] is also their order. Removed entries are unlinked
/// from `prev`/`next`, never reused.
#[derive(Debug)]
struct Delim {
    node: usize,
    ch: char,
    /// Delimiter characters not used yet
    count: usize,
    /// Length of the delimiter run, for the "rule of 3"
    len: usize,
    can_open: bool,
    can_close: bool,
    prev: Option<usize>,
    next: Option<usize>,
}

#[derive(Debug)]
struct Bracket {
    node: usize,
    /// Cleared once a link is made, as links can't contain links
    active: bool,
    /// Last entry of the delimiter list when the bracket was found
    delim_bottom: Option<usize>,
}

struct InlineParser<'a> {
    input: &'a [NoteChar],
    pos: usize,
    nodes: Vec<String>,
    /// The last node is plain text that more text can be added to
    text_open: bool,
    /// Ranges of nodes `[start, end)` with a format
    spans: Vec<(usize, usize, Format)>,
    /// Ranges of nodes `[start, end)` in a link, with the destination
    links: Vec<(usize, usize, String)>,
    delims: Vec<Delim>,
    last_delim: Option<usize>,
    brackets: Vec<Bracket>,
}

impl InlineParser<'_> {
    fn parse(&mut self) {
        while let Some(&ch) = self.input.get(self.pos) {
            if ch.escaped {
                self.push_text(ch.c);
                self.pos += 1;
                continue;
            }
            match ch.c {
                '*' | '_' => self.delimiter_run(ch.c),
                '[' => {
                    let node = self.push_node("[".to_string());
                    self.brackets.push(Bracket {
                        node,
                        active: true,
                        delim_bottom: self.last_delim,
                    });
                    self.pos += 1;
                }
                ']' => self.close_bracket(),
                c => {
                    self.push_text(c);
                    self.pos += 1;
                }
            }
        }
        self.process_emphasis(None);
    }

    fn push_text(&mut self, c: char) {
        match self.nodes.last_mut() {
            Some(last) if self.text_open => last.push(c),
            _ => {
                self.nodes.push(c.to_string());
                self.text_open = true;
            }
        }
    }

    fn push_node(&mut self, text: String) -> usize {
        self.nodes.push(text);
        self.text_open = false;
        self.nodes.len() - 1
    }

    fn get(&self, i: usize) -> Option<char> {
        self.input.get(i).map(|c| c.c)
    }

    /// Checks for an unescaped `c` at `i`
    fn is(&self, i: usize, c: char) -> bool {
        self.input.get(i).is_some_and(|x| x.c == c && !x.escaped)
    }

    fn delimiter_run(&mut self, ch: char) {
        let start = self.pos;
        while self.is(self.pos, ch) {
            self.pos += 1;
        }
        let len = self.pos - start;

        // the start and the end of the text count as whitespace
        let before = start.checked_sub(1).and_then(|i| self.get(i));
        let after = self.get(self.pos);
        let ws = |c: Option<char>| c.is_none_or(is_whitespace);
        let punct = |c: Option<char>| c.is_some_and(is_punctuation);

        let left_flanking = !ws(after) && (!punct(after) || ws(before) || punct(before));
        let right_flanking = !ws(before) && (!punct(before) || ws(after) || punct(after));
        let (can_open, can_close) = if ch == '*' {
            (left_flanking, right_flanking)
        } else {
            (
                left_flanking && (!right_flanking || punct(before)),
                right_flanking && (!left_flanking || punct(after)),
            )
        };

        let node = self.push_node(std::iter::repeat_n(ch, len).collect());
        if can_open || can_close {
            let index = self.delims.len();
            self.delims.push(Delim {
                node,
                ch,
                count: len,
                len,
                can_open,
                can_close,
                prev: self.last_delim,
                next: None,
            });
            if let Some(last) = self.last_delim {
                self.delims[last].next = Some(index);
            }
            self.last_delim = Some(index);
        }
    }

    fn remove_delim(&mut self, i: usize) {
        let (prev, next) = (self.delims[i].prev, self.delims[i].next);
        if let Some(p) = prev {
            self.delims[p].next = next;
        }
        match next {
            Some(n) => self.delims[n].prev = prev,
            None => self.last_delim = prev,
        }
    }

    /// The "process emphasis" procedure of `CommonMark`, for the delimiters
    /// after `bottom` (all of them if `None`)
    fn process_emphasis(&mut self, bottom: Option<usize>) {
        let above = |i: usize, limit: Option<usize>| limit.is_none_or(|b| i > b);

        // lowest delimiter to look at for an opener, by delimiter char,
        // length of the closer modulo 3 and whether the closer can open
        let mut openers_bottom = [[[bottom; 2]; 3]; 2];

        let mut closer = None;
        let mut cur = self.last_delim;
        while let Some(i) = cur.filter(|&i| above(i, bottom)) {
            closer = Some(i);
            cur = self.delims[i].prev;
        }

        while let Some(ci) = closer {
            let c = &self.delims[ci];
            if !c.can_close {
                closer = c.next;
                continue;
            }
            let slot = (usize::from(c.ch == '_'), c.len % 3, usize::from(c.can_open));
            let limit = openers_bottom[slot.0][slot.1][slot.2];

            let mut found = None;
            let mut cur = c.prev;
            while let Some(oi) = cur.filter(|&i| above(i, limit)) {
                let o = &self.delims[oi];
                if o.can_open && o.ch == c.ch {
                    let either_both = c.can_open || o.can_close;
                    let rule_of_3 = !(o.len + c.len).is_multiple_of(3)
                        || (o.len.is_multiple_of(3) && c.len.is_multiple_of(3));
                    if !either_both || rule_of_3 {
                        found = Some(oi);
                        break;
                    }
                }
                cur = o.prev;
            }

            match found {
                Some(oi) => {
                    let used = if self.delims[oi].count >= 2 && self.delims[ci].count >= 2 {
                        2
                    } else {
                        1
                    };
                    let format = if used == 2 {
                        Format::Strong
                    } else {
                        Format::Emphasis
                    };
                    let (open_node, close_node) = (self.delims[oi].node, self.delims[ci].node);
                    self.spans.push((open_node + 1, close_node, format));
                    for (d, node) in [(oi, open_node), (ci, close_node)] {
                        self.delims[d].count -= used;
                        let n = &mut self.nodes[node];
                        n.truncate(n.len() - used);
                    }
                    // delimiters in between can't match anymore
                    let mut cur = self.delims[ci].prev;
                    while let Some(i) = cur.filter(|&i| i != oi) {
                        cur = self.delims[i].prev;
                        self.remove_delim(i);
                    }
                    if self.delims[oi].count == 0 {
                        self.remove_delim(oi);
                    }
                    if self.delims[ci].count == 0 {
                        closer = self.delims[ci].next;
                        self.remove_delim(ci);
                    }
                }
                None => {
                    openers_bottom[slot.0][slot.1][slot.2] = self.delims[ci].prev.or(bottom);
                    closer = self.delims[ci].next;
                    if !self.delims[ci].can_open {
                        self.remove_delim(ci);
                    }
                }
            }
        }

        while let Some(i) = self.last_delim.filter(|&i| above(i, bottom)) {
            self.remove_delim(i);
        }
    }

    fn close_bracket(&mut self) {
        let Some(opener) = self.brackets.last() else {
            self.push_text(']');
            self.pos += 1;
            return;
        };
        if !opener.active {
            self.brackets.pop();
            self.push_text(']');
            self.pos += 1;
            return;
        }
        match self.link_tail(self.pos + 1) {
            Some((destination, end)) => {
                let opener = self.brackets.pop().unwrap();
                self.process_emphasis(opener.delim_bottom);
                self.links
                    .push((opener.node + 1, self.nodes.len(), destination));
                self.nodes[opener.node].clear();
                for b in &mut self.brackets {
                    b.active = false;
                }
                self.text_open = false;
                self.pos = end;
            }
            None => {
                self.brackets.pop();
                self.push_text(']');
                self.pos += 1;
            }
        }
    }

    fn skip_whitespace(&self, mut i: usize) -> usize {
        while self
            .get(i)
            .is_some_and(|c| matches!(c, ' ' | '\t' | '\n' | '\r'))
        {
            i += 1;
        }
        i
    }

    /// Reads `(destination "title")` starting at `i`, giving the destination
    /// and the index after the `)`
    fn link_tail(&self, i: usize) -> Option<(String, usize)> {
        if !self.is(i, '(') {
            return None;
        }
        let mut i = self.skip_whitespace(i + 1);
        let mut destination = String::new();

        if self.is(i, '<') {
            i += 1;
            loop {
                let x = self.input.get(i)?;
                if !x.escaped && x.c == '>' {
                    i += 1;
                    break;
                }
                if !x.escaped && matches!(x.c, '<' | '\n') {
                    return None;
                }
                destination.push(x.c);
                i += 1;
            }
        } else {
            let mut depth = 0;
            while let Some(x) = self.input.get(i) {
                if x.c == ' ' || x.c.is_ascii_control() {
                    break;
                }
                if !x.escaped && x.c == '(' {
                    depth += 1;
                    if depth > MAX_NESTED_PARENS {
                        return None;
                    }
                }
                if !x.escaped && x.c == ')' {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                }
                destination.push(x.c);
                i += 1;
            }
            if depth != 0 {
                return None;
            }
        }

        let after_destination = i;
        i = self.skip_whitespace(i);
        // a title, which is accepted and discarded
        if i > after_destination && (self.is(i, '"') || self.is(i, '\'') || self.is(i, '(')) {
            let open = self.input[i].c;
            let close = if open == '(' { ')' } else { open };
            i += 1;
            loop {
                let x = self.input.get(i)?;
                if !x.escaped && x.c == close {
                    i += 1;
                    break;
                }
                if !x.escaped && open == '(' && x.c == '(' {
                    return None;
                }
                i += 1;
            }
            i = self.skip_whitespace(i);
        }

        self.is(i, ')').then_some((destination, i + 1))
    }

    fn into_runs(self) -> Vec<TextRun> {
        let n = self.nodes.len();
        let mut strong = vec![0i32; n + 1];
        let mut emphasis = vec![0i32; n + 1];
        for &(start, end, format) in &self.spans {
            let counts = match format {
                Format::Strong => &mut strong,
                Format::Emphasis => &mut emphasis,
            };
            counts[start] += 1;
            counts[end] -= 1;
        }
        // links never nest nor overlap
        let mut link_of = vec![None; n];
        for (i, &(start, end, _)) in self.links.iter().enumerate() {
            link_of[start..end].fill(Some(i));
        }

        let mut runs: Vec<TextRun> = Vec::new();
        let (mut in_strong, mut in_emphasis) = (0, 0);
        for (i, text) in self.nodes.into_iter().enumerate() {
            in_strong += strong[i];
            in_emphasis += emphasis[i];
            if text.is_empty() {
                continue;
            }
            let run = TextRun {
                value: text,
                strong: in_strong > 0,
                emphasis: in_emphasis > 0,
                link: link_of[i].map(|l| self.links[l].2.clone()),
            };
            match runs.last_mut() {
                Some(last) if last.same_format(&run) => last.value += &run.value,
                _ => runs.push(run),
            }
        }
        runs
    }
}

/// Unicode whitespace as `CommonMark` defines it
fn is_whitespace(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0C' | '\r') || c.is_separator_space()
}

/// Unicode punctuation as `CommonMark` 0.31.2 defines it: the P and S general
/// categories
fn is_punctuation(c: char) -> bool {
    c.is_punctuation() || c.is_symbol()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    /// Builds the input from a string where `\x` is an escaped `x`, as the
    /// lexer does
    fn chars(s: &str) -> Vec<NoteChar> {
        let mut out = Vec::new();
        let mut it = s.chars();
        while let Some(c) = it.next() {
            if c == '\\' {
                if let Some(n) = it.next() {
                    out.push(NoteChar {
                        c: n,
                        escaped: true,
                    });
                }
            } else {
                out.push(NoteChar { c, escaped: false });
            }
        }
        out
    }

    /// Compact form of the runs: plain text as is, `<b>`/`<i>` for strong
    /// and emphasis, `<a dest>` around a run in a link
    fn show(runs: &[TextRun]) -> Vec<String> {
        runs.iter()
            .map(|r| {
                let mut s = r.value.clone();
                if r.emphasis {
                    s = format!("<i>{s}</i>");
                }
                if r.strong {
                    s = format!("<b>{s}</b>");
                }
                match &r.link {
                    Some(dest) => format!("<a {dest}>{s}</a>"),
                    None => s,
                }
            })
            .collect()
    }

    /// Parses and, when the input has no escapes and `CommonMark` reads it with
    /// the same subset, checks that `CommonMark` agrees
    fn p(s: &str) -> Vec<String> {
        let got = parse_note(&chars(s));
        if !s.contains('\\') {
            if let Some(expected) = differential::oracle(s) {
                assert_eq!(show(&got), show(&expected), "differs from `CommonMark`");
            }
        }
        show(&got)
    }

    // The examples in the proposal
    #[test_case("Optional, but *highly* suggested." => vec!["Optional, but ", "<i>highly</i>", " suggested."] ; "emphasis")]
    #[test_case("***both***" => vec!["<b><i>both</i></b>"] ; "strong and emphasis")]
    #[test_case("See [**Basic** dough](<./Basic dough.cook>)" => vec!["See ", "<a ./Basic dough.cook><b>Basic</b></a>", "<a ./Basic dough.cook> dough</a>"] ; "formatted link text")]
    #[test_case("*Use fine sea salt" => vec!["*Use fine sea salt"] ; "lone asterisk")]
    #[test_case("Save it as tomato_sauce_v2.cook" => vec!["Save it as tomato_sauce_v2.cook"] ; "intraword underscore")]
    #[test_case(r"This is \*not\* emphasis" => vec!["This is *not* emphasis"] ; "escaped asterisks")]
    #[test_case("**unclosed bold and *closed italic*" => vec!["**unclosed bold and ", "<i>closed italic</i>"] ; "unclosed strong")]
    #[test_case("[tips] (see below)" => vec!["[tips] (see below)"] ; "brackets without destination")]
    #[test_case("Add @salt{1%tsp}" => vec!["Add @salt{1%tsp}"] ; "cooklang markers")]
    #[test_case("**Make ahead:** the dough keeps *up to 3 days* in the fridge. See the [shaping guide](https://example.com/shaping) before you start." => vec!["<b>Make ahead:</b>", " the dough keeps ", "<i>up to 3 days</i>", " in the fridge. See the ", "<a https://example.com/shaping>shaping guide</a>", " before you start."] ; "proposal example")]
    #[test_case("**Serve *immediately* or not at all**" => vec!["<b>Serve </b>", "<b><i>immediately</i></b>", "<b> or not at all</b>"] ; "nested")]
    #[test_case(r"Use 2\*250 g tins, not 3\*100 g jars." => vec!["Use 2*250 g tins, not 3*100 g jars."] ; "escaped multiplication")]
    #[test_case("2*250 g tins or 3*100 g jars" => vec!["2", "<i>250 g tins or 3</i>", "100 g jars"] ; "unescaped multiplication")]
    #[test_case("* Day-old rice works best" => vec!["* Day-old rice works best"] ; "hand written bullet")]
    #[test_case("a * b * c" => vec!["a * b * c"] ; "spaced asterisks")]
    #[test_case("[1] and [tips]" => vec!["[1] and [tips]"] ; "footnote brackets")]
    #[test_case("![photo](dough.jpg)" => vec!["!", "<a dough.jpg>photo</a>"] ; "image is a link")]
    #[test_case("This filling also works in [Spinach pie](<./Spinach pie.cook>)." => vec!["This filling also works in ", "<a ./Spinach pie.cook>Spinach pie</a>", "."] ; "recipe link")]
    #[test_case("Makes *about* 1 and 1/4 cups filling." => vec!["Makes ", "<i>about</i>", " 1 and 1/4 cups filling."] ; "existing markdown")]
    #[test_case(r"https://example.com/a\--b" => vec!["https://example.com/a--b"] ; "escaped dash")]
    fn proposal(input: &str) -> Vec<String> {
        p(input)
    }

    #[test_case("" => Vec::<String>::new() ; "empty")]
    #[test_case("just text" => vec!["just text"] ; "plain")]
    #[test_case("_a_ b" => vec!["<i>a</i>", " b"] ; "underscore emphasis")]
    #[test_case("__a__ b" => vec!["<b>a</b>", " b"] ; "underscore strong")]
    #[test_case("un*believ*able" => vec!["un", "<i>believ</i>", "able"] ; "intraword asterisk")]
    #[test_case("*foo**bar**baz*" => vec!["<i>foo</i>", "<b><i>bar</i></b>", "<i>baz</i>"] ; "strong inside emphasis")]
    #[test_case("**foo*" => vec!["*", "<i>foo</i>"] ; "extra opener")]
    #[test_case("*foo**" => vec!["<i>foo</i>", "*"] ; "extra closer")]
    #[test_case("*foo**bar*" => vec!["<i>foo**bar</i>"] ; "rule of three")]
    #[test_case("[a]()" => vec!["<a >a</a>"] ; "empty destination")]
    #[test_case("[a](<>)" => vec!["<a >a</a>"] ; "empty angle destination")]
    #[test_case("[a]( b )" => vec!["<a b>a</a>"] ; "spaces around destination")]
    #[test_case(r#"[a](b "title")"# => vec!["<a b>a</a>"] ; "double quoted title")]
    #[test_case("[a](b 'title')" => vec!["<a b>a</a>"] ; "single quoted title")]
    #[test_case("[a](b (title))" => vec!["<a b>a</a>"] ; "parenthesised title")]
    #[test_case(r#"[a](b"title")"# => vec![r#"<a b"title">a</a>"#] ; "quote in bare destination")]
    #[test_case(r#"[a](b "title)"# => vec![r#"[a](b "title)"#] ; "unclosed title")]
    #[test_case("[a](b (ti(tle))" => vec!["[a](b (ti(tle))"] ; "parenthesis inside parenthesised title")]
    #[test_case(r"[a](b (ti\(tle))" => vec!["<a b>a</a>"] ; "escaped parenthesis inside parenthesised title")]
    #[test_case(r#"[a](b "ti\"tle")"# => vec!["<a b>a</a>"] ; "escaped quote inside title")]
    #[test_case("[a](<b c>)" => vec!["<a b c>a</a>"] ; "angle destination with space")]
    #[test_case("[a](b c)" => vec!["[a](b c)"] ; "space in bare destination")]
    #[test_case("[a](https://example.com/a_(b))" => vec!["<a https://example.com/a_(b)>a</a>"] ; "balanced parentheses")]
    #[test_case("[a](b(c)" => vec!["[a](b(c)"] ; "unbalanced parentheses")]
    #[test_case(r"[a](b\)c)" => vec!["<a b)c>a</a>"] ; "escaped parenthesis in destination")]
    #[test_case(r"[a](<b\>c>)" => vec!["<a b>c>a</a>"] ; "escaped angle in destination")]
    #[test_case("[a](<b>c>)" => vec!["[a](<b>c>)"] ; "text after angle destination")]
    #[test_case("[a](<b<c>)" => vec!["[a](<b<c>)"] ; "angle inside angle destination")]
    #[test_case("[a](b" => vec!["[a](b"] ; "unclosed destination")]
    #[test_case(r"\[a](b)" => vec!["[a](b)"] ; "escaped open bracket")]
    #[test_case(r"[a\](b)" => vec!["[a](b)"] ; "escaped close bracket")]
    #[test_case(r"[a]\(b)" => vec!["[a](b)"] ; "escaped open parenthesis")]
    #[test_case("[a [b](c) d](e)" => vec!["[a ", "<a c>b</a>", " d](e)"] ; "no link inside link")]
    #[test_case("[a [b] c](d)" => vec!["<a d>a [b] c</a>"] ; "brackets inside link text")]
    #[test_case("*[a*](u)" => vec!["*", "<a u>a*</a>"] ; "link binds tighter than emphasis")]
    #[test_case("*[a](u)*" => vec!["<a u><i>a</i></a>"] ; "emphasis around link")]
    #[test_case("[a](u)[b](u)" => vec!["<a u>ab</a>"] ; "adjacent links merge")]
    #[test_case("[*a*](u) [b](v)" => vec!["<a u><i>a</i></a>", " ", "<a v>b</a>"] ; "two links")]
    #[test_case("**a** **b**" => vec!["<b>a</b>", " ", "<b>b</b>"] ; "two strong")]
    #[test_case(r"\**a**" => vec!["*", "<i>a</i>", "*"] ; "escaped first delimiter")]
    #[test_case(r"a\\*b*" => vec![r"a\", "<i>b</i>"] ; "escaped backslash")]
    #[test_case("_a_b_" => vec!["<i>a_b</i>"] ; "underscore inside word")]
    #[test_case("@_name_" => vec!["@", "<i>name</i>"] ; "handle in underscores")]
    #[test_case("(*) a (**) b (***) c" => vec!["(", "<i>) a (</i>", "<b><i>) b (</i></b>", ") c"] ; "footnote marks")]
    #[test_case("**" => vec!["**"] ; "only delimiters")]
    #[test_case("a ** b" => vec!["a ** b"] ; "spaced double")]
    #[test_case("a *b\n*" => vec!["a *b\n*"] ; "newline is whitespace")]
    fn formatting(input: &str) -> Vec<String> {
        p(input)
    }

    #[test]
    fn runs_merge_and_are_not_empty() {
        let runs = parse_note(&chars("a*b*c"));
        assert!(runs.iter().all(|r| !r.value.is_empty()));
        for w in runs.windows(2) {
            assert!(!w[0].same_format(&w[1]));
        }
    }

    #[test]
    fn plain_text_is_kept() {
        for s in ["a *b* c", "[x](y) z", "**", "*a", r"\*a\*"] {
            let input = chars(s);
            let plain: String = parse_note(&input)
                .iter()
                .map(|r| r.value.as_str())
                .collect();
            let without_formatting: String = input.iter().map(|c| c.c).collect();
            // formatting only removes delimiters and destinations
            assert!(plain.len() <= without_formatting.len());
        }
    }

    #[test]
    fn deep_nesting_does_not_overflow() {
        let s = "[".repeat(10_000) + &"*a".repeat(10_000) + &"]".repeat(10_000);
        let runs = parse_note(&chars(&s));
        assert!(!runs.is_empty());
        let s = "*a ".repeat(20_000) + &"b* ".repeat(20_000);
        let _ = parse_note(&chars(&s));
    }

    /// Compare with a `CommonMark` implementation on inputs where the subset
    /// Cooklang reads is the whole of what `CommonMark` reads
    mod differential {
        use super::*;
        use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

        /// Runs from `CommonMark`, or `None` if the input uses something
        /// notes do not read (images, HTML, code, breaks...)
        pub(super) fn oracle(input: &str) -> Option<Vec<TextRun>> {
            let mut runs: Vec<TextRun> = Vec::new();
            let (mut strong, mut emphasis) = (0u32, 0u32);
            let mut link: Option<String> = None;
            let mut paragraphs = 0;
            for ev in Parser::new_ext(input, Options::empty()) {
                match ev {
                    Event::Start(Tag::Paragraph) => paragraphs += 1,
                    Event::End(TagEnd::Paragraph) => {}
                    Event::Start(Tag::Emphasis) => emphasis += 1,
                    Event::End(TagEnd::Emphasis) => emphasis -= 1,
                    Event::Start(Tag::Strong) => strong += 1,
                    Event::End(TagEnd::Strong) => strong -= 1,
                    Event::Start(Tag::Link {
                        link_type: pulldown_cmark::LinkType::Inline,
                        dest_url,
                        ..
                    }) => link = Some(dest_url.into_string()),
                    Event::End(TagEnd::Link) => link = None,
                    Event::Text(t) => {
                        let run = TextRun {
                            value: t.into_string(),
                            strong: strong > 0,
                            emphasis: emphasis > 0,
                            link: link.clone(),
                        };
                        match runs.last_mut() {
                            Some(last) if last.same_format(&run) => last.value += &run.value,
                            _ => runs.push(run),
                        }
                    }
                    _ => return None,
                }
            }
            if paragraphs != 1 {
                return None;
            }
            Some(runs)
        }

        struct Rng(u64);
        impl Rng {
            fn next(&mut self) -> u64 {
                // xorshift64
                self.0 ^= self.0 << 13;
                self.0 ^= self.0 >> 7;
                self.0 ^= self.0 << 17;
                self.0
            }
        }

        #[test]
        fn matches_commonmark() {
            const ALPHABET: &[char] = &[
                '*', '*', '_', '_', '[', ']', '(', ')', '<', '>', '"', '\'', ' ', 'a', 'b', '.',
                'é', '-', ':', '/',
            ];
            let mut rng = Rng(0x2545F4914F6CDD1D);
            let mut compared = 0;
            let mut failures = Vec::new();
            for _ in 0..300_000 {
                let len = 1 + (rng.next() % 14) as usize;
                let mut s = String::from("x ");
                for _ in 0..len {
                    s.push(ALPHABET[(rng.next() % ALPHABET.len() as u64) as usize]);
                }
                // trailing spaces are dropped by markdown paragraphs
                if s.ends_with(' ') {
                    continue;
                }
                let Some(expected) = oracle(&s) else {
                    continue;
                };
                compared += 1;
                let got = parse_note(&chars(&s));
                if got != expected && failures.len() < 20 {
                    failures.push(format!(
                        "{s:?}\n   got {:?}\n  want {:?}",
                        show(&got),
                        show(&expected)
                    ));
                }
            }
            assert!(compared > 100_000, "only {compared} cases compared");
            assert!(failures.is_empty(), "{}", failures.join("\n"));
        }
    }
}
