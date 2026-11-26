mod common;

use common::*;
use gregory_core::*;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;

const POS: f64 = 1e-9;
const NA_NB: f64 = 1e-6;
const G1_DEG: f64 = 1e-3;

#[derive(Deserialize)]
struct Input {
    v: Vec<V3>,
    f: Vec<Vec<usize>>,
    tau: Num,
    mode: String,
    n: usize,
    creases: Vec<[usize; 2]>,
}
#[derive(Deserialize)]
struct JEdge {
    a: usize,
    b: usize,
    faces: Vec<JFace>,
    e: [V3; 4],
    crease: bool,
}
#[derive(Deserialize)]
struct JFace {
    fi: usize,
    k: usize,
}
#[derive(Deserialize)]
struct JMesh {
    pos: Vec<V3>,
    nrm: Vec<V3>,
    tris: Vec<[usize; 3]>,
    tri_face: Vec<usize>,
}
#[derive(Deserialize)]
struct JSample {
    p: V3,
    na: V3,
    nb: V3,
}
#[derive(Deserialize)]
struct JCheck {
    g0: Num,
    g1: Num,
    pairs: usize,
    creased: usize,
    samples: Vec<JSample>,
}
#[derive(Deserialize)]
struct JCount {
    q: usize,
    t: usize,
}
#[derive(Deserialize)]
struct JEd {
    e: [V3; 4],
    c: [V3; 4],
}
#[derive(Deserialize)]
struct JEval {
    a: Num,
    b: Num,
    at: V3,
    normal: V3,
}
#[derive(Deserialize)]
struct JEdgePoint {
    k: usize,
    t: Num,
    h: Num,
    p: [V3; 2],
}
#[derive(Deserialize)]
struct JPatch {
    kind: String,
    ed: Vec<JEd>,
    q: Vec<[V3; 2]>,
    evals: Vec<JEval>,
    edge_points: Vec<JEdgePoint>,
}
#[derive(Deserialize)]
struct Output {
    nrm: Vec<V3>,
    edges: Vec<JEdge>,
    mesh: JMesh,
    check: JCheck,
    count: JCount,
    patches: Vec<JPatch>,
}
#[derive(Deserialize)]
struct Case {
    name: String,
    input: Input,
    output: Option<Output>,
    error: Option<String>,
}

fn load_all() -> Vec<Case> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/build-model");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|p| {
            serde_json::from_str(&std::fs::read_to_string(p).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()))
        })
        .collect()
}

fn run(input: &Input) -> Result<Model, GregoryError> {
    let opts = BuildOptions {
        tau: input.tau.get(),
        mode: Mode::from_js_name(&input.mode),
        n: input.n,
        creases: input
            .creases
            .iter()
            .map(|&[a, b]| EdgeKey::new(a, b))
            .collect::<HashSet<_>>(),
    };
    build_model(&v3s(&input.v), &input.f, &opts)
}

struct Diff {
    name: String,
    msgs: Vec<String>,
}

impl Diff {
    fn num(&mut self, what: &str, got: f64, want: f64, tol: f64) {
        if !close(got, want, tol) {
            self.msgs.push(format!(
                "{what}: got {got:?}, want {want:?}, diff {:e}",
                (got - want).abs()
            ));
        }
    }
    fn vec(&mut self, what: &str, got: [f64; 3], want: &V3, tol: f64) {
        let w = v3(want);
        for k in 0..3 {
            if !close(got[k], w[k], tol) {
                self.msgs.push(format!(
                    "{what}[{k}]: got {:?}, want {:?}, diff {:e}",
                    got[k],
                    w[k],
                    (got[k] - w[k]).abs()
                ));
            }
        }
    }
    fn vecs(&mut self, what: &str, got: &[[f64; 3]], want: &[V3], tol: f64) {
        if got.len() != want.len() {
            self.msgs.push(format!(
                "{what}: 件数 got {}, want {}",
                got.len(),
                want.len()
            ));
            return;
        }
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            self.vec(&format!("{what}[{i}]"), *g, w, tol);
        }
    }
    fn exact<T: PartialEq + std::fmt::Debug>(&mut self, what: &str, got: T, want: T) {
        if got != want {
            self.msgs
                .push(format!("{what}: got {got:?}, want {want:?}"));
        }
    }
}

fn compare(case: &Case, m: &Model, want: &Output) -> Diff {
    let mut d = Diff {
        name: case.name.clone(),
        msgs: vec![],
    };
    d.vecs("nrm", &m.nrm, &want.nrm, POS);

    d.exact("edges.len", m.edges.len(), want.edges.len());
    for (i, (g, w)) in m.edges.iter().zip(&want.edges).enumerate() {
        d.exact(&format!("edges[{i}].a"), g.a, w.a);
        d.exact(&format!("edges[{i}].b"), g.b, w.b);
        d.exact(&format!("edges[{i}].crease"), g.crease, w.crease);
        let faces: Vec<_> = g.faces.iter().map(|f| (f.fi, f.k)).collect();
        let wfaces: Vec<_> = w.faces.iter().map(|f| (f.fi, f.k)).collect();
        d.exact(&format!("edges[{i}].faces"), faces, wfaces);
        for k in 0..4 {
            d.vec(&format!("edges[{i}].e[{k}]"), g.e[k], &w.e[k], POS);
        }
    }

    let vm: Vec<[f64; 3]> = m.mesh.pos.clone();
    d.vecs("mesh.pos", &vm, &want.mesh.pos, POS);
    d.vecs("mesh.nrm", &m.mesh.nrm, &want.mesh.nrm, POS);
    d.exact("mesh.tris", m.mesh.tris.clone(), want.mesh.tris.clone());
    d.exact(
        "mesh.tri_face",
        m.mesh.tri_face.clone(),
        want.mesh.tri_face.clone(),
    );

    d.num("check.g0", m.check.g0, want.check.g0.get(), POS);
    d.num("check.g1", m.check.g1, want.check.g1.get(), G1_DEG);
    d.exact("check.pairs", m.check.pairs, want.check.pairs);
    d.exact("check.creased", m.check.creased, want.check.creased);
    d.exact(
        "check.samples.len",
        m.check.samples.len(),
        want.check.samples.len(),
    );
    for (i, (g, w)) in m.check.samples.iter().zip(&want.check.samples).enumerate() {
        d.vec(&format!("samples[{i}].p"), g.p, &w.p, POS);
        d.vec(&format!("samples[{i}].na"), g.na, &w.na, NA_NB);
        d.vec(&format!("samples[{i}].nb"), g.nb, &w.nb, NA_NB);
    }
    d.exact("count.q", m.count.q, want.count.q);
    d.exact("count.t", m.count.t, want.count.t);

    d.exact("patches.len", m.patches.len(), want.patches.len());
    for (pi, (p, w)) in m.patches.iter().zip(&want.patches).enumerate() {
        let kind = if p.kind() == PatchKind::Quad {
            "quad"
        } else {
            "tri"
        };
        d.exact(&format!("patches[{pi}].kind"), kind, w.kind.as_str());
        d.exact(&format!("patches[{pi}].ed.len"), p.ed().len(), w.ed.len());
        for (k, (g, we)) in p.ed().iter().zip(&w.ed).enumerate() {
            for j in 0..4 {
                d.vec(
                    &format!("patches[{pi}].ed[{k}].e[{j}]"),
                    g.e[j],
                    &we.e[j],
                    POS,
                );
                d.vec(
                    &format!("patches[{pi}].ed[{k}].c[{j}]"),
                    g.c[j],
                    &we.c[j],
                    POS,
                );
            }
        }
        d.exact(&format!("patches[{pi}].q.len"), p.q().len(), w.q.len());
        for (k, (g, wq)) in p.q().iter().zip(&w.q).enumerate() {
            d.vec(&format!("patches[{pi}].q[{k}][0]"), g[0], &wq[0], POS);
            d.vec(&format!("patches[{pi}].q[{k}][1]"), g[1], &wq[1], POS);
        }
        for (i, ev) in w.evals.iter().enumerate() {
            let (a, b) = (ev.a.get(), ev.b.get());
            d.vec(
                &format!("patches[{pi}].evals[{i}].at"),
                p.at(a, b),
                &ev.at,
                POS,
            );
            d.vec(
                &format!("patches[{pi}].evals[{i}].normal"),
                p.normal(a, b),
                &ev.normal,
                POS,
            );
        }
        for (i, ep) in w.edge_points.iter().enumerate() {
            match p.edge_point(ep.k, ep.t.get(), ep.h.get()) {
                Some(got) => {
                    d.vec(
                        &format!("patches[{pi}].edge_points[{i}].p[0]"),
                        got[0],
                        &ep.p[0],
                        POS,
                    );
                    d.vec(
                        &format!("patches[{pi}].edge_points[{i}].p[1]"),
                        got[1],
                        &ep.p[1],
                        POS,
                    );
                }
                None => d.msgs.push(format!("patches[{pi}].edge_points[{i}]: None")),
            }
        }
    }
    d
}

#[test]
fn every_case_matches_js_output() {
    let cases = load_all();
    assert!(cases.len() >= 50, "期待値が足りない: {}", cases.len());
    let mut failures = Vec::new();
    for case in &cases {
        let result = run(&case.input);
        match (&case.output, &case.error, result) {
            (Some(want), None, Ok(m)) => {
                let d = compare(case, &m, want);
                if !d.msgs.is_empty() {
                    let shown: Vec<_> = d.msgs.iter().take(8).cloned().collect();
                    failures.push(format!(
                        "[{}] 不一致 {} 件\n  {}",
                        d.name,
                        d.msgs.len(),
                        shown.join("\n  ")
                    ));
                }
            }
            (Some(_), None, Err(e)) => {
                failures.push(format!("[{}] JS は成功したが Rust は Err: {e}", case.name))
            }
            (None, Some(js), Ok(_)) => {
                failures.push(format!("[{}] JS は例外 ({js}) だが Rust は Ok", case.name))
            }
            (None, Some(_), Err(_)) => {}
            _ => failures.push(format!("[{}] 期待値の形が不正", case.name)),
        }
    }
    assert!(
        failures.is_empty(),
        "{} / {} 件が不一致\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn error_cases_return_the_expected_error_kind() {
    let find = |name: &str| {
        load_all()
            .into_iter()
            .find(|c| c.name == name)
            .unwrap_or_else(|| panic!("{name} がない"))
    };
    let err = |name: &str| {
        run(&find(name).input)
            .err()
            .unwrap_or_else(|| panic!("{name} が Ok"))
    };
    assert!(matches!(
        err("error-vertex-out-of-range"),
        GregoryError::VertexOutOfRange { .. }
    ));
    for name in [
        "error-face-two-vertices",
        "error-face-one-vertex",
        "error-face-empty",
    ] {
        assert!(
            matches!(err(name), GregoryError::TooFewCorners { .. }),
            "{name}"
        );
    }
    assert!(matches!(
        err("error-pentagon-pair-shared-fourth-edge"),
        GregoryError::EdgeOutsidePatch { .. }
    ));
}
