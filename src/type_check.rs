#![warn(warnings)]

use codespan_reporting::Diagnostic;
use crate::hir;
use crate::ir::DefId;
use crate::map::FxIndexMap;
use crate::parser::Span;
use crate::ty;
use crate::ty::base_inferred::BaseInferred;
use crate::ty::base_only::{BaseOnly, BaseTy};
use crate::ty::declaration::Declaration;
use crate::ty::interners::{HasTyInternTables, TyInternTables};
use crate::ty::map_family::Map;
use crate::ty::BaseData;
use crate::ty::Generics;
use crate::ty::Ty;
use crate::ty::TypeFamily;
use crate::unify::InferVar;
use crate::unify::Inferable;
use crate::unify::UnificationTable;
use generational_arena::Arena;
use std::sync::Arc;

mod base_only;
mod hir_typeck;
mod ops;
mod query_definitions;

salsa::query_group! {
    crate trait TypeCheckDatabase: hir::HirDatabase + HasTyInternTables {
        /// Compute the "base type information" for a given fn body.
        /// This is the type information excluding permissions.
        fn base_type_check(key: DefId) -> TypeCheckResults<BaseInferred> {
            type BaseTypeCheckQuery;
            use fn query_definitions::base_type_check;
        }
    }
}

struct TypeChecker<'db, DB: TypeCheckDatabase, F: TypeCheckFamily> {
    db: &'db DB,
    hir: Arc<hir::FnBody>,
    ops_arena: Arena<Box<dyn ops::BoxedTypeCheckerOp<Self>>>,
    ops_blocked: FxIndexMap<InferVar, Vec<ops::OpIndex>>,
    unify: UnificationTable<TyInternTables, hir::MetaIndex>,
    results: TypeCheckResults<F>,
}

trait TypeCheckFamily: TypeFamily {
    type TcBase: From<Self::Base>
        + Into<Self::Base>
        + Inferable<TyInternTables, KnownData = ty::BaseData<Self>>;

    fn new_infer_ty(this: &mut impl TypeCheckerFields<Self>) -> Ty<Self>;

    fn equate_types(
        this: &mut impl TypeCheckerFields<Self>,
        cause: hir::MetaIndex,
        ty1: Ty<Self>,
        ty2: Ty<Self>,
    );

    fn boolean_type(this: &impl TypeCheckerFields<Self>) -> Ty<Self>;

    fn error_type(this: &impl TypeCheckerFields<Self>) -> Ty<Self>;

    fn require_assignable(
        this: &mut impl TypeCheckerFields<Self>,
        expression: hir::Expression,
        value_ty: Ty<Self>,
        place_ty: Ty<Self>,
    );

    fn apply_user_perm(
        this: &mut impl TypeCheckerFields<Self>,
        perm: hir::Perm,
        place_ty: Ty<Self>,
    ) -> Ty<Self>;

    fn least_upper_bound(
        this: &mut impl TypeCheckerFields<Self>,
        if_expression: hir::Expression,
        true_ty: Ty<Self>,
        false_ty: Ty<Self>,
    ) -> Ty<Self>;

    // FIXME -- This *almost* could be done generically but that
    // `Substitution` currently requires that `Perm = Erased`; we'll
    // have to push the "perm combination" into `TypeFamily` or
    // something.  Cross that bridge when we come to it.
    fn substitute<M>(
        this: &mut impl TypeCheckerFields<Self>,
        location: hir::MetaIndex,
        owner_perm: Self::Perm,
        owner_base_data: &BaseData<Self>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, Self>;
}

trait TypeCheckerFields<F: TypeCheckFamily>: HasTyInternTables {
    type DB: TypeCheckDatabase;

    fn db(&self) -> &Self::DB;
    fn unify(&mut self) -> &mut UnificationTable<TyInternTables, hir::MetaIndex>;
    fn results(&mut self) -> &mut TypeCheckResults<F>;
}

impl<'me, DB, F> TypeCheckerFields<F> for TypeChecker<'me, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeCheckFamily,
{
    type DB = DB;

    fn db(&self) -> &DB {
        &self.db
    }

    fn unify(&mut self) -> &mut UnificationTable<TyInternTables, hir::MetaIndex> {
        &mut self.unify
    }

    fn results(&mut self) -> &mut TypeCheckResults<F> {
        &mut self.results
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct TypeCheckResults<F: TypeFamily> {
    /// FIXME-- this will actually not want `BaseTy` unless we want to
    /// return the unification table too.
    types: std::collections::BTreeMap<hir::MetaIndex, Ty<F>>,

    errors: Vec<Error>,
}

impl<F: TypeFamily> TypeCheckResults<F> {
    fn record_ty(&mut self, index: impl Into<hir::MetaIndex>, ty: Ty<F>) {
        self.types.insert(index.into(), ty);
    }

    crate fn ty(&self, index: impl Into<hir::MetaIndex>) -> Ty<F> {
        self.types[&index.into()]
    }

    fn record_error(&mut self, location: impl Into<hir::MetaIndex>) {
        self.errors.push(Error {
            location: location.into(),
        });
    }
}

impl<F: TypeFamily> Default for TypeCheckResults<F> {
    fn default() -> Self {
        Self {
            types: Default::default(),
            errors: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
crate struct Error {
    location: hir::MetaIndex,
}

impl<DB, F> HasTyInternTables for TypeChecker<'_, DB, F>
where
    DB: TypeCheckDatabase,
    F: TypeCheckFamily,
{
    fn ty_intern_tables(&self) -> &TyInternTables {
        self.db.ty_intern_tables()
    }
}