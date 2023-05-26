pub struct Recipe(pub(crate) cooklang::Recipe);

pub struct Metadata(pub(crate) cooklang::metadata::Metadata);

pub struct Ast<'a>(pub(crate) cooklang::ast::Ast<'a>);
