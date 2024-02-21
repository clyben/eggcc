use std::iter;

use strum::IntoEnumIterator;

use crate::schema_helpers::{Constructor, Purpose};

fn is_inv_base_case_for_ctor(ctor: Constructor) -> Option<String> {
    let br = "\n      ";
    let ruleset = " :ruleset always-run";

    match ctor {
        // I assume input is tuple here
        // TODO
        Constructor::Get => Some(format!(
            "(rule ((BodyContainsExpr loop expr) \
			{br} (= loop (DoWhile in out)) \
            {br} (= expr (Get (Arg ty) i)) \
            {br} (= expr (DoWhile-outputs-ith loop i))) \
            {br}((set (is-inv-Expr loop expr) true)){ruleset})"
        )),
        Constructor::Const => {
            let ctor_pattern = ctor.construct(|field| field.var());
            Some(format!(
                "(rule ((BodyContainsExpr loop expr) \
                {br} (= loop (DoWhile in out)) \
                {br} (= expr {ctor_pattern})) \
                {br}((set (is-inv-Expr loop expr) true)){ruleset})"
            ))
        }
        _ => None,
    }
}

fn is_invariant_rule_for_ctor(ctor: Constructor) -> Option<String> {
    let br = "\n      ";
    let ruleset = " :ruleset always-run";
    let ctor_pattern = ctor.construct(|field| field.var());

    match ctor {
        // list handled in loop_invariant.egg
        // base cases are skipped
        // print, load, and Write are not invariant
        Constructor::Cons
        | Constructor::Nil
        | Constructor::Const
        | Constructor::Arg
        | Constructor::Alloc => None, // TODO
        _ => {
            let is_inv_ctor = ctor
                .filter_map_fields(|field| match field.purpose {
                    Purpose::Static(_) | Purpose::CapturedExpr => None,
                    Purpose::SubExpr | Purpose::SubListExpr => {
                        let var = field.var();
                        let sort = field.sort().name();
                        Some(format!("(= true (is-inv-{sort} loop {var}))"))
                    }
                })
                .join(" ");
            let is_pure = match ctor {
                Constructor::Call | Constructor::Let | Constructor::DoWhile => {
                    format!("{br} (ExprIsPure expr)")
                }
                _ => String::new(),
            };

            let op_is_pure = match ctor {
                Constructor::Bop => "(BinaryOpIsPure _op)",
                Constructor::Uop => "(UnaryOpIsPure _op)",
                _ => "",
            };

            Some(format!(
                "(rule ((BodyContainsExpr loop expr) \
                {br} (= loop (DoWhile in out)) \
                {br} (= expr {ctor_pattern}) {op_is_pure} \
                {br} {is_inv_ctor} {is_pure}) \
                {br}((set (is-inv-Expr loop expr) true)){ruleset})"
            ))
        }
    }
}

pub(crate) fn rules() -> Vec<String> {
    iter::once(include_str!("loop_invariant.egg").to_string())
        .chain(Constructor::iter().filter_map(is_inv_base_case_for_ctor))
        .chain(Constructor::iter().filter_map(is_invariant_rule_for_ctor))
        .collect::<Vec<_>>()
}
