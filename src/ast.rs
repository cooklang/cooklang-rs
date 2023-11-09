use serde::Serialize;

use crate::{
    error::{PassResult, SourceReport},
    parser::{Block, BlockKind, Event, Item},
};

/// Abstract syntax tree of a cooklang file
///
/// The AST is (mostly) borrowed from the input and offers location information of each
/// element back to the source file.
#[derive(Debug, Serialize, Clone)]
pub struct Ast<'a> {
    pub blocks: Vec<Block<'a>>,
}

/// Builds an [`Ast`] given an [`Event`] iterator
///
/// Probably the iterator you want is an instance of [`PullParser`](crate::parser::PullParser).
#[tracing::instrument(level = "debug", skip_all)]
pub fn build_ast<'input>(events: impl Iterator<Item = Event<'input>>) -> PassResult<Ast<'input>> {
    let mut blocks = Vec::new();
    let mut items = Vec::new();
    let mut ctx = SourceReport::empty();
    for event in events {
        match event {
            Event::Metadata { key, value } => blocks.push(Block::Metadata { key, value }),
            Event::Section { name } => blocks.push(Block::Section { name }),
            Event::Start(_kind) => items.clear(),
            Event::End(kind) => {
                match kind {
                    BlockKind::Step => {
                        if !items.is_empty() {
                            blocks.push(Block::Step {
                                items: std::mem::take(&mut items),
                            })
                        }
                    }
                    BlockKind::Text => {
                        let texts = std::mem::take(&mut items)
                            .into_iter()
                            .map(|i| {
                                if let Item::Text(t) = i {
                                    t
                                } else {
                                    panic!("Not text in text block: {i:?}");
                                }
                            })
                            .collect();
                        blocks.push(Block::TextBlock(texts))
                    }
                };
            }
            Event::Text(t) => items.push(Item::Text(t)),
            Event::Ingredient(c) => items.push(Item::Ingredient(Box::new(c))),
            Event::Cookware(c) => items.push(Item::Cookware(Box::new(c))),
            Event::Timer(c) => items.push(Item::Timer(Box::new(c))),
            Event::Error(e) => ctx.push(e),
            Event::Warning(w) => ctx.push(w),
        }
    }
    let ast = Ast { blocks };
    PassResult::new(Some(ast), ctx)
}
