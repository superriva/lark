#![feature(const_fn)]
#![feature(const_let)]
#![feature(crate_visibility_modifier)]
#![feature(macro_at_most_once_rep)]
#![feature(in_band_lifetimes)]
#![feature(specialization)]
#![feature(try_blocks)]
#![allow(dead_code)]

use crate::lexer::token::LexToken;

use lark_debug_derive::DebugWith;
use lark_entity::{Entity, EntityTables};
use lark_error::{Diagnostic, WithError};
use lark_seq::Seq;
use lark_span::{FileName, Span, Spanned};
use lark_string::{GlobalIdentifierTables, Text};

pub mod current_file;
mod ir;
mod lexer;
mod macros;
mod parser;
mod query_definitions;
pub mod syntax;

pub use self::ir::ParsedFile;
pub use self::syntax::entity::ParsedEntity;
pub use self::syntax::uhir;

salsa::query_group! {
    pub trait ParserDatabase: AsRef<GlobalIdentifierTables>
        + AsRef<EntityTables>
        + salsa::Database
    {
        fn file_names() -> Seq<FileName> {
            type FileNamesQuery;
            storage input;
        }

        fn file_text(id: FileName) -> Text {
            type FileTextQuery;
            storage input;
        }

        // FIXME: In general, this is wasteful of space, and not
        // esp. incremental friendly. It would be better store
        // e.g. the length of each token only, so that we can adjust
        // the previous value (not to mention perhaps using a rope or
        // some other similar data structure that permits insertions).
        fn file_tokens(id: FileName) -> WithError<Seq<Spanned<LexToken, FileName>>> {
            type FileTokensQuery;
            use fn query_definitions::file_tokens;
        }

        fn parsed_file(id: FileName) -> WithError<ParsedFile> {
            type RootEntitiesQuery;
            use fn query_definitions::parsed_file;
        }

        fn child_parsed_entities(entity: Entity) -> WithError<Seq<ParsedEntity>> {
            type ChildParsedEntitiesQuery;
            use fn query_definitions::child_parsed_entities;
        }

        fn parsed_entity(entity: Entity) -> WithError<ParsedEntity> {
            type ParsedEntityQuery;
            use fn query_definitions::parsed_entity;
        }

        fn child_entities(entity: Entity) -> Seq<Entity> {
            type ChildEntitiesQuery;
            use fn query_definitions::child_entities;
        }

        fn uhir_of_entity(entity: Entity) -> WithError<uhir::Entity> {
            type UhirOfEntityQuery;
            use fn query_definitions::uhir_of_entity;
        }
    }
}

fn diagnostic(message: impl Into<String>, span: Span<FileName>) -> Diagnostic {
    Diagnostic::new(message.into(), span)
}
