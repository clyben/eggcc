#[cfg(test)]
use crate::{
    ast::*,
    egglog_test,
    interpreter::{Pointer, Value},
    schema::{RcExpr, Type},
};

#[cfg(test)]
fn type_test(inp: RcExpr, expected_ty: Type, arg: Value, expected_val: Value) -> crate::Result {
    let build = format!("{inp}");
    let check = format!("(check (HasType {inp} {expected_ty}))");
    egglog_test(
        &build,
        &check,
        vec![inp.to_program(emptyt(), expected_ty)],
        arg,
        expected_val,
    )
}

#[cfg(test)]
fn type_error_test(inp: RcExpr) {
    let _ = egglog_test(&format!("{inp}"), "", vec![], val_empty(), val_empty());
}

#[cfg(test)]
fn _debug(before: &str, after: &str) -> crate::Result {
    egglog_test(before, after, vec![], val_empty(), val_empty())
}

#[test]
fn primitives() -> crate::Result {
    type_test(int(3), intt(), val_int(0), val_int(3))?;
    type_test(int(12), intt(), val_int(0), val_int(12))?;
    type_test(ttrue(), boolt(), val_int(0), val_bool(true))?;
    type_test(tfalse(), boolt(), val_int(0), val_bool(false))?;
    type_test(empty(), emptyt(), val_int(0), val_empty())
}

#[test]
fn uops() -> crate::Result {
    let m = int(3);
    let x = ttrue();
    let y = tfalse();
    type_test(not(x), boolt(), val_int(0), val_bool(false))?;
    type_test(not(y), boolt(), val_int(0), val_bool(true))?;
    type_test(tprint(m), emptyt(), val_int(0), val_empty())
}

#[test]
#[should_panic]
fn not_error() {
    type_error_test(not(int(4)));
}

#[test]
#[should_panic]
fn load_error() {
    type_error_test(load(int(4)));
}

#[test]
fn bops() -> crate::Result {
    let m = int(3);
    let n = int(12);
    type_test(add(m.clone(), n.clone()), intt(), val_int(0), val_int(15))?;
    type_test(sub(m.clone(), n.clone()), intt(), val_int(0), val_int(-9))?;
    type_test(
        mul(
            add(m.clone(), m.clone()),
            sub(add(n.clone(), n.clone()), m.clone()),
        ),
        intt(),
        val_int(0),
        val_int(126),
    )
}

#[test]
#[should_panic]
fn add_error() {
    type_error_test(add(int(4), ttrue()));
}

#[test]
#[should_panic]
fn sub_error() {
    type_error_test(sub(tfalse(), ttrue()));
}

#[test]
#[should_panic]
fn mul_error() {
    type_error_test(mul(less_than(int(4), int(5)), int(3)));
}

#[test]
#[should_panic]
fn less_than_error() {
    type_error_test(less_than(less_than(int(4), int(5)), int(3)));
}

#[test]
#[should_panic]
fn and_error() {
    type_error_test(and(ttrue(), and(tfalse(), int(2))));
}

#[test]
#[should_panic]
fn or_error() {
    type_error_test(or(tfalse(), int(2)));
}

#[test]
fn pointers() -> crate::Result {
    let ptr = alloc(int(12), intt());
    type_test(
        ptr.clone(),
        pointert(intt()),
        val_int(0),
        Value::Ptr(Pointer::new(0, 12, 0)),
    )?;
    type_test(
        write(ptr.clone(), ttrue()),
        emptyt(),
        val_int(0),
        val_empty(),
    )?;
    type_test(
        ptradd(alloc(int(1), boolt()), add(int(1), int(2))),
        emptyt(),
        val_int(0),
        Value::Ptr(Pointer::new(0, 1, 3)),
    )
}

#[test]
#[should_panic]
fn pointer_type_error() {
    type_error_test(alloc(less_than(int(1), int(2)), boolt()));
}

#[test]
fn tuple() -> crate::Result {
    type_test(
        single(int(30)),
        tuplet_vec(vec![intt()]),
        val_int(0),
        val_vec(vec![val_int(30)]),
    )?;

    type_test(
        concat_par(single(int(20)), single(ttrue())),
        tuplet_vec(vec![intt(), boolt()]),
        val_int(0),
        val_vec(vec![val_int(20), val_bool(true)]),
    )
}

#[test]
fn ifs() -> crate::Result {
    type_test(tif(ttrue(), int(1), int(2)), intt(), val_int(0), val_int(1))?;

    type_test(
        tif(
            less_than(int(2), int(3)),
            and(ttrue(), tfalse()),
            or(less_than(int(3), int(4)), ttrue()),
        ),
        boolt(),
        val_int(0),
        val_bool(false),
    )
}

#[test]
#[should_panic]
fn if_pred() {
    type_error_test(tif(int(1), int(2), int(3)));
}

#[test]
#[should_panic]
fn if_branches() {
    type_error_test(tif(ttrue(), int(2), tfalse()));
}

#[test]
fn switches() -> crate::Result {
    type_test(
        switch_vec(int(1), vec![int(0), int(21)]),
        intt(),
        val_int(0),
        val_int(21),
    )?;
    type_test(
        switch_vec(int(0), vec![ttrue()]),
        boolt(),
        val_int(0),
        val_bool(true),
    )?;
    type_test(
        switch_vec(int(2), vec![int(1), int(2), int(3), int(4)]),
        intt(),
        val_int(0),
        val_int(3),
    )
}

#[test]
#[should_panic]
fn switch_pred() {
    type_error_test(switch_vec(ttrue(), vec![int(1), int(2)]));
}

#[test]
#[should_panic]
fn switch_branches() {
    type_error_test(switch_vec(int(1), vec![ttrue(), int(1)]));
}

#[test]
fn lets() -> crate::Result {
    let inp = tlet(int(4), add(arg(intt()), arg(intt())));
    type_test(inp, intt(), val_int(0), val_int(8))
}

#[test]
#[should_panic]
fn let_type_error() {
    type_error_test(tlet(int(1), and(arg(boolt()), ttrue())));
}