mod common;

use common::*;
use gregory_core::*;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Deserialize)]
struct JModel {
    v: Vec<V3>,
    f: Vec<Vec<usize>>,
    creases: Vec<[usize; 2]>,
}

#[derive(Deserialize)]
struct JMid {
    edge: [usize; 2],
    p: V3,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Args {
    op: Option<String>,
    key: Option<String>,
    model: Option<JModel>,
    v: Option<Vec<V3>>,
    f: Option<Value>,
    face_idxs: Vec<usize>,
    dist: Option<Num>,
    edge_keys: Vec<[usize; 2]>,
    mid_of: Option<Vec<JMid>>,
    mode: Option<String>,
    from: Option<String>,
    to: Option<String>,
    items: Vec<Value>,
    eps: Option<Num>,
    steps: Vec<Args>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    op: String,
    input: Args,
    output: Option<Value>,
    error: Option<String>,
    undefined_output: Option<bool>,
}

#[derive(Deserialize)]
struct JEntry {
    a: usize,
    b: usize,
    faces: Vec<usize>,
}
#[derive(Deserialize)]
struct JCompact {
    model: JModel,
    remap: Vec<Option<usize>>,
}
#[derive(Deserialize)]
struct JExtrude {
    model: JModel,
    faces: Vec<usize>,
}
#[derive(Deserialize)]
struct JLoop {
    model: JModel,
    edges: Vec<[usize; 2]>,
}
#[derive(Deserialize)]
struct JPreset {
    name: String,
    model: JModel,
}

enum Out {
    Preset(Preset),
    EdgeMap(EdgeMap),
    Normal(Vec3),
    Compact(Compacted),
    Model(CageModel),
    Extrude(Extruded),
    Loop(LoopCut),
    Selection(Selection),
    Verts(Vec<usize>),
    Mirror(Vec<Option<usize>>),
    Problems(Vec<String>),
}

impl Out {
    fn model(&self) -> &CageModel {
        match self {
            Out::Preset(_) => unreachable!(),
            Out::Compact(c) => &c.model,
            Out::Model(m) => m,
            Out::Extrude(e) => &e.model,
            Out::Loop(l) => &l.model,
            _ => panic!("モデルを返さない操作を sequence で使っている"),
        }
    }
}

fn to_cage(m: &JModel) -> CageModel {
    CageModel {
        v: v3s(&m.v),
        f: m.f.clone(),
        creases: m.creases.iter().map(|&[a, b]| EdgeKey::new(a, b)).collect(),
    }
}

fn mode_items(mode: SelectMode, items: &[Value]) -> Selection {
    let num = |x: &Value| x.as_u64().expect("非負整数") as usize;
    let key = |x: &Value| {
        let a = x.as_array().expect("辺は配列");
        EdgeKey::new(num(&a[0]), num(&a[1]))
    };
    match mode {
        SelectMode::Vertex => Selection::Vertices(items.iter().map(num).collect()),
        SelectMode::Edge => Selection::Edges(items.iter().map(key).collect()),
        SelectMode::Face => Selection::Faces(items.iter().map(num).collect()),
    }
}

fn run_op(op: &str, a: &Args, prev: Option<&CageModel>) -> Result<Out, String> {
    let model_in = || -> CageModel {
        match prev {
            Some(m) => m.clone(),
            None => to_cage(a.model.as_ref().expect("model")),
        }
    };
    let keys = || -> Vec<EdgeKey> {
        a.edge_keys
            .iter()
            .map(|&[x, y]| EdgeKey::new(x, y))
            .collect()
    };
    let err = |e: MeshError| format!("{e:?}");
    Ok(match op {
        "presets" => Out::Preset(Preset::from_key(a.key.as_deref().unwrap()).ok_or("未知のキー")?),
        "edge_map" => {
            let f: Vec<Vec<usize>> = serde_json::from_value(a.f.clone().unwrap()).unwrap();
            Out::EdgeMap(edge_map(&f))
        }
        "face_normal" => {
            let f: Vec<usize> = serde_json::from_value(a.f.clone().unwrap()).unwrap();
            Out::Normal(face_normal(&v3s(a.v.as_ref().unwrap()), &f).map_err(err)?)
        }
        "compact" => Out::Compact(compact(&model_in()).map_err(err)?),
        "delete_faces" => Out::Model(delete_faces(&model_in(), &a.face_idxs).map_err(err)?),
        "extrude_faces" => Out::Extrude(
            extrude_faces(&model_in(), &a.face_idxs, a.dist.as_ref().unwrap().get())
                .map_err(err)?,
        ),
        "loop_cut" => {
            let mids: Option<HashMap<EdgeKey, Vec3>> = a.mid_of.as_ref().map(|l| {
                l.iter()
                    .map(|m| (EdgeKey::new(m.edge[0], m.edge[1]), v3(&m.p)))
                    .collect()
            });
            Out::Loop(loop_cut(&model_in(), &keys(), mids.as_ref()).map_err(err)?)
        }
        "toggle_creases" => Out::Model(toggle_creases(&model_in(), &keys())),
        "selection_vertices" => {
            let mode = SelectMode::from_js_name(a.mode.as_deref().unwrap());
            Out::Verts(selection_vertices(&model_in(), &mode_items(mode, &a.items)))
        }
        "convert_selection" => {
            let from = SelectMode::from_js_name(a.from.as_deref().unwrap());
            let to = SelectMode::from_js_name(a.to.as_deref().unwrap());
            Out::Selection(convert_selection(
                &model_in(),
                &mode_items(from, &a.items),
                to,
            ))
        }
        "mirror_map" => Out::Mirror(mirror_map(
            &v3s(a.v.as_ref().unwrap()),
            a.eps.as_ref().unwrap().get(),
        )),
        "validate_model" => Out::Problems(
            validate_model(&model_in())
                .iter()
                .map(ToString::to_string)
                .collect(),
        ),
        other => panic!("未知の op: {other}"),
    })
}

struct Diff(Vec<String>);

impl Diff {
    fn eq<T: PartialEq + std::fmt::Debug>(&mut self, what: &str, got: &T, want: &T) {
        if got != want {
            self.0.push(format!("{what}: 実際={got:?} 期待={want:?}"));
        }
    }

    fn model(&mut self, what: &str, got: &CageModel, want: &JModel) {
        let wv = v3s(&want.v);
        if got.v.len() != wv.len() {
            self.0.push(format!(
                "{what}.v の長さ: 実際={} 期待={}",
                got.v.len(),
                wv.len()
            ));
        } else {
            for (i, (g, w)) in got.v.iter().zip(&wv).enumerate() {
                if !same3(*g, *w) {
                    self.0.push(format!("{what}.v[{i}]: 実際={g:?} 期待={w:?}"));
                }
            }
        }
        self.eq(&format!("{what}.f"), &got.f, &want.f);
        let wc: HashSet<EdgeKey> = want
            .creases
            .iter()
            .map(|&[a, b]| EdgeKey::new(a, b))
            .collect();
        self.eq(&format!("{what}.creases"), &got.creases, &wc);
    }

    fn compare(&mut self, what: &str, got: &Out, want: &Value) {
        match got {
            Out::Preset(p) => {
                let w: JPreset = serde_json::from_value(want.clone()).unwrap();
                self.eq(&format!("{what}.name"), &p.name(), &w.name.as_str());
                self.model(&format!("{what}.model"), &p.make(), &w.model);
            }
            Out::EdgeMap(m) => {
                let w: Vec<JEntry> = serde_json::from_value(want.clone()).unwrap();
                let g: Vec<(usize, usize, Vec<usize>)> = m
                    .entries()
                    .iter()
                    .map(|e| (e.a, e.b, e.faces.iter().map(|c| c.fi).collect()))
                    .collect();
                let w: Vec<(usize, usize, Vec<usize>)> =
                    w.into_iter().map(|e| (e.a, e.b, e.faces)).collect();
                self.eq(what, &g, &w);
            }
            Out::Normal(n) => {
                let w: V3 = serde_json::from_value(want.clone()).unwrap();
                if !same3(*n, v3(&w)) {
                    self.0.push(format!("{what}: 実際={n:?} 期待={:?}", v3(&w)));
                }
            }
            Out::Compact(c) => {
                let w: JCompact = serde_json::from_value(want.clone()).unwrap();
                self.model(&format!("{what}.model"), &c.model, &w.model);
                self.eq(&format!("{what}.remap"), &c.remap, &w.remap);
            }
            Out::Model(m) => {
                let w: JModel = serde_json::from_value(want.clone()).unwrap();
                self.model(what, m, &w);
            }
            Out::Extrude(e) => {
                let w: JExtrude = serde_json::from_value(want.clone()).unwrap();
                self.model(&format!("{what}.model"), &e.model, &w.model);
                self.eq(&format!("{what}.faces"), &e.faces, &w.faces);
            }
            Out::Loop(l) => {
                let w: JLoop = serde_json::from_value(want.clone()).unwrap();
                self.model(&format!("{what}.model"), &l.model, &w.model);
                let g: Vec<[usize; 2]> = l.edges.iter().map(|k| [k.a(), k.b()]).collect();
                self.eq(&format!("{what}.edges"), &g, &w.edges);
            }
            Out::Selection(s) => match s {
                Selection::Vertices(x) | Selection::Faces(x) => {
                    let w: Vec<usize> = serde_json::from_value(want.clone()).unwrap();
                    self.eq(what, x, &w);
                }
                Selection::Edges(x) => {
                    let w: Vec<[usize; 2]> = serde_json::from_value(want.clone()).unwrap();
                    let g: Vec<[usize; 2]> = x.iter().map(|k| [k.a(), k.b()]).collect();
                    self.eq(what, &g, &w);
                }
            },
            Out::Verts(x) => {
                let w: Vec<usize> = serde_json::from_value(want.clone()).unwrap();
                self.eq(what, x, &w);
            }
            Out::Mirror(m) => {
                let w: Vec<i64> = serde_json::from_value(want.clone()).unwrap();
                let w: Vec<Option<usize>> = w
                    .into_iter()
                    .map(|x| (x >= 0).then_some(x as usize))
                    .collect();
                self.eq(what, m, &w);
            }
            Out::Problems(p) => {
                let w: Vec<String> = serde_json::from_value(want.clone()).unwrap();
                self.eq(what, p, &w);
            }
        }
    }
}

fn load_all() -> Vec<Case> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mesh");
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

fn run_case(c: &Case) -> Vec<String> {
    let mut d = Diff(Vec::new());
    let expects_err = c.error.is_some() || c.undefined_output == Some(true);
    if c.op == "sequence" {
        let want: Vec<Value> = serde_json::from_value(c.output.clone().expect("output")).unwrap();
        let mut prev: Option<Out> = None;
        if want.len() != c.input.steps.len() {
            d.0.push("steps と output の長さが違う".into());
            return d.0;
        }
        for (k, (step, w)) in c.input.steps.iter().zip(&want).enumerate() {
            let model = prev.as_ref().map(Out::model);
            let step_op = step.op.as_deref().unwrap();
            // 最初の段は input.model、以降は前段の出力モデルが使われる
            let out = match run_op(step_op, &merge(&c.input, step), model) {
                Ok(o) => o,
                Err(e) => {
                    d.0.push(format!("steps[{k}] が Err: {e}"));
                    return d.0;
                }
            };
            d.compare(&format!("steps[{k}].{step_op}"), &out, w);
            prev = Some(out);
        }
        return d.0;
    }
    match run_op(&c.op, &c.input, None) {
        Err(e) => {
            if !expects_err {
                d.0.push(format!("期待は Ok だが Err: {e}"));
            }
        }
        Ok(out) => {
            if expects_err {
                d.0.push("期待は Err（error / undefined_output）だが Ok".into());
            } else {
                match &c.output {
                    Some(w) => d.compare("output", &out, w),
                    None => d.0.push("output がない".into()),
                }
            }
        }
    }
    d.0
}

// 最初の段は input.model を持つ入力に、その段の引数を重ねる
fn merge(base: &Args, step: &Args) -> Args {
    Args {
        op: step.op.clone(),
        model: base.model.as_ref().map(|m| JModel {
            v: m.v.clone(),
            f: m.f.clone(),
            creases: m.creases.clone(),
        }),
        face_idxs: step.face_idxs.clone(),
        dist: step.dist.clone(),
        edge_keys: step.edge_keys.clone(),
        mid_of: step.mid_of.as_ref().map(|l| {
            l.iter()
                .map(|m| JMid {
                    edge: m.edge,
                    p: m.p.clone(),
                })
                .collect()
        }),
        ..Args::default()
    }
}

#[test]
fn すべての期待値と一致する() {
    let cases = load_all();
    assert_eq!(cases.len(), 219, "期待値の件数");
    let mut failed = Vec::new();
    for c in &cases {
        let msgs = run_case(c);
        if !msgs.is_empty() {
            failed.push(format!("[{}] ({})\n  {}", c.name, c.op, msgs.join("\n  ")));
        }
    }
    assert!(
        failed.is_empty(),
        "{} 件が不一致:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

#[test]
fn 名前の接頭辞と結果の種類が一致する() {
    for c in load_all() {
        let kind = (c.error.is_some(), c.undefined_output == Some(true));
        let want = (
            c.name.starts_with("error-"),
            c.name.starts_with("undefined-"),
        );
        assert_eq!(kind, want, "{}", c.name);
        assert_eq!(c.output.is_some(), !kind.0 && !kind.1, "{}", c.name);
    }
}
