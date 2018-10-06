use crate::hir;
use crate::hir::HirDatabase;
use crate::ir::DefId;
use crate::ty;
use crate::ty::base_only::{Base, BaseOnly, BaseTy};
use crate::ty::declaration::Declaration;
use crate::ty::interners::HasTyInternTables;
use crate::ty::interners::TyInternTables;
use crate::ty::map_family::Map;
use crate::ty::substitute::Substitution;
use crate::ty::Erased;
use crate::ty::InferVarOr;
use crate::ty::Signature;
use crate::ty::Ty;
use crate::ty::TypeFamily;
use crate::ty::{BaseData, BaseKind};
use crate::ty::{Generic, Generics};
use crate::type_check::Error;
use crate::type_check::TypeCheckFamily;
use crate::type_check::TypeChecker;
use crate::type_check::TypeCheckerFields;
use crate::unify::{InferVar, UnificationTable};
use std::sync::Arc;

impl TypeCheckFamily for BaseOnly {
    type TcBase = Base;

    fn new_infer_ty(this: &mut impl TypeCheckerFields<Self>) -> Ty<Self> {
        Ty {
            perm: Erased,
            base: this.unify().new_inferable(),
        }
    }

    fn equate_types(
        this: &mut impl TypeCheckerFields<Self>,
        cause: hir::MetaIndex,
        ty1: Ty<BaseOnly>,
        ty2: Ty<BaseOnly>,
    ) {
        let Ty {
            perm: Erased,
            base: base1,
        } = ty1;
        let Ty {
            perm: Erased,
            base: base2,
        } = ty2;

        match this.unify().unify(cause, base1, base2) {
            Ok(()) => {}

            Err((data1, data2)) => {
                if data1.kind != data2.kind {
                    this.results().errors.push(Error { location: cause });
                    return;
                }

                for (generic1, generic2) in data1.generics.iter().zip(&data2.generics) {
                    match (generic1, generic2) {
                        (Generic::Ty(g1), Generic::Ty(g2)) => {
                            Self::equate_types(this, cause, g1, g2);
                        }
                    }
                }
            }
        }
    }

    fn boolean_type(this: &impl TypeCheckerFields<Self>) -> BaseTy {
        let boolean_def_id = this.db().boolean_def_id(());
        Ty {
            perm: Erased,
            base: BaseOnly::intern_base_data(
                this.db(),
                BaseData {
                    kind: BaseKind::Named(boolean_def_id),
                    generics: Generics::empty(),
                },
            ),
        }
    }

    fn error_type(this: &impl TypeCheckerFields<Self>) -> BaseTy {
        Ty {
            perm: Erased,
            base: BaseOnly::intern_base_data(
                this.db(),
                BaseData {
                    kind: BaseKind::Error,
                    generics: Generics::empty(),
                },
            ),
        }
    }

    fn apply_user_perm(
        _this: &mut impl TypeCheckerFields<Self>,
        _perm: hir::Perm,
        place_ty: Ty<BaseOnly>,
    ) -> Ty<BaseOnly> {
        // In the "erased type check", we don't care about permissions.
        place_ty
    }

    fn require_assignable(
        this: &mut impl TypeCheckerFields<Self>,
        expression: hir::Expression,
        value_ty: Ty<BaseOnly>,
        place_ty: Ty<BaseOnly>,
    ) {
        Self::equate_types(this, expression.into(), value_ty, place_ty)
    }

    fn least_upper_bound(
        this: &mut impl TypeCheckerFields<Self>,
        if_expression: hir::Expression,
        true_ty: Ty<BaseOnly>,
        false_ty: Ty<BaseOnly>,
    ) -> Ty<BaseOnly> {
        Self::equate_types(this, if_expression.into(), true_ty, false_ty);
        true_ty
    }

    fn substitute<M>(
        this: &mut impl TypeCheckerFields<Self>,
        _location: hir::MetaIndex,
        _owner_perm: Erased,
        owner_base_data: &BaseData<Self>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, Self>,
    {
        value.map(&mut Substitution::new(this, &owner_base_data.generics))
    }
}