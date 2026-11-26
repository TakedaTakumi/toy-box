mod common;

use common::*;
use gregory_core::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Bin {
    a: V3,
    b: V3,
    result: V3,
}
#[derive(Deserialize)]
struct BinS {
    a: V3,
    b: V3,
    result: Num,
}
#[derive(Deserialize)]
struct Mul {
    a: V3,
    s: Num,
    result: V3,
}
#[derive(Deserialize)]
struct Un {
    a: V3,
    result: Num,
}
#[derive(Deserialize)]
struct UnV {
    a: V3,
    result: V3,
}
#[derive(Deserialize)]
struct Bez {
    e: [V3; 4],
    t: Num,
    result: V3,
}
#[derive(Deserialize)]
struct Fixture {
    add: Vec<Bin>,
    sub: Vec<Bin>,
    dot: Vec<BinS>,
    cross: Vec<Bin>,
    mul: Vec<Mul>,
    len: Vec<Un>,
    norm: Vec<UnV>,
    bezier3: Vec<Bez>,
}

fn load() -> Fixture {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/vector.json"
    ))
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn check_bin(name: &str, cases: &[Bin], f: fn(Vec3, Vec3) -> Vec3) {
    assert!(!cases.is_empty());
    for c in cases {
        let (a, b) = (v3(&c.a), v3(&c.b));
        let got = f(a, b);
        assert!(
            same3(got, v3(&c.result)),
            "{name}({a:?}, {b:?}) = {got:?}, 期待 {:?}",
            v3(&c.result)
        );
    }
}

#[test]
fn add_matches_js_bitwise() {
    check_bin("add", &load().add, add);
}

#[test]
fn sub_matches_js_bitwise() {
    check_bin("sub", &load().sub, sub);
}

#[test]
fn cross_matches_js_bitwise() {
    check_bin("cross", &load().cross, cross);
}

#[test]
fn dot_matches_js_bitwise() {
    for c in &load().dot {
        let (a, b) = (v3(&c.a), v3(&c.b));
        assert!(
            same(dot(a, b), c.result.get()),
            "dot({a:?}, {b:?}) = {}, 期待 {}",
            dot(a, b),
            c.result.get()
        );
    }
}

#[test]
fn mul_matches_js_bitwise() {
    for c in &load().mul {
        let (a, s) = (v3(&c.a), c.s.get());
        assert!(
            same3(mul(a, s), v3(&c.result)),
            "mul({a:?}, {s}) = {:?}, 期待 {:?}",
            mul(a, s),
            v3(&c.result)
        );
    }
}

#[test]
fn len_matches_js_bitwise() {
    for c in &load().len {
        let a = v3(&c.a);
        assert!(
            same(len(a), c.result.get()),
            "len({a:?}) = {:?}, 期待 {:?}",
            len(a),
            c.result.get()
        );
    }
}

#[test]
fn norm_matches_js_bitwise() {
    for c in &load().norm {
        let a = v3(&c.a);
        assert!(
            same3(norm(a), v3(&c.result)),
            "norm({a:?}) = {:?}, 期待 {:?}",
            norm(a),
            v3(&c.result)
        );
    }
}

#[test]
fn bezier3_matches_js_bitwise() {
    let cases = load().bezier3;
    assert!(!cases.is_empty());
    for c in &cases {
        let (e, t) = (v4(&c.e), c.t.get());
        assert!(
            same3(bezier3(&e, t), v3(&c.result)),
            "bezier3(t={t}) = {:?}, 期待 {:?}",
            bezier3(&e, t),
            v3(&c.result)
        );
    }
}
