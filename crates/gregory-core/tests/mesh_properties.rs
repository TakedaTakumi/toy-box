use gregory_core::*;
use proptest::prelude::*;
use std::collections::{HashMap, HashSet};

// ---- 生成器 ----

fn any_preset() -> impl Strategy<Value = Preset> {
    prop::sample::select(PRESETS.to_vec())
}

#[derive(Clone, Debug)]
struct Op {
    kind: usize,
    pick: Vec<usize>,
    dist: f64,
}

fn any_op() -> impl Strategy<Value = Op> {
    (
        0..4usize,
        prop::collection::vec(0..1000usize, 1..=3),
        -0.6..=0.8f64,
    )
        .prop_map(|(kind, pick, dist)| Op { kind, pick, dist })
}

fn dedup(xs: impl IntoIterator<Item = usize>) -> Vec<usize> {
    let mut seen = HashSet::new();
    xs.into_iter().filter(|x| seen.insert(*x)).collect()
}

// kind: 0 押し出し・1 ループ分割・2 削除・3 折れ目
fn apply(m: &CageModel, o: &Op, kind: usize) -> CageModel {
    let nf = m.f.len();
    let faces = dedup(o.pick.iter().map(|p| p % nf));
    match kind {
        0 => extrude_faces(m, &faces, o.dist).unwrap().model,
        2 if nf <= 1 => m.clone(),
        2 => delete_faces(m, &faces[..faces.len().min(nf - 1)]).unwrap(),
        _ => {
            let keys: Vec<EdgeKey> = edge_map(&m.f).keys().collect();
            let picked: Vec<EdgeKey> = dedup(o.pick.iter().map(|p| p % keys.len()))
                .into_iter()
                .map(|i| keys[i])
                .collect();
            if kind == 1 {
                loop_cut(m, &picked, None).unwrap().model
            } else {
                toggle_creases(m, &picked)
            }
        }
    }
}

fn apply_all(p: Preset, ops: &[Op], force: Option<usize>) -> CageModel {
    ops.iter()
        .fold(p.make(), |m, o| apply(&m, o, force.unwrap_or(o.kind)))
}

fn build(m: &CageModel, n: usize) -> Model {
    let opts = BuildOptions {
        tau: 1.0,
        mode: Mode::G1,
        n,
        creases: m.creases.clone(),
    };
    build_model(&m.v, &m.f, &opts).unwrap()
}

fn finite_mesh(b: &Model) -> bool {
    b.mesh
        .pos
        .iter()
        .chain(&b.mesh.nrm)
        .all(|p| p.iter().all(|x| x.is_finite()))
}

fn problems(m: &CageModel) -> Vec<String> {
    validate_model(m).iter().map(ToString::to_string).collect()
}

/// NaN を含んでもビット単位で比べられる形にする。
type Snapshot = (Vec<[u64; 3]>, Vec<Vec<usize>>, Vec<(usize, usize)>);

fn snapshot(m: &CageModel) -> Snapshot {
    let mut c: Vec<(usize, usize)> = m.creases.iter().map(|k| (k.a(), k.b())).collect();
    c.sort_unstable();
    (
        m.v.iter().map(|p| p.map(f64::to_bits)).collect(),
        m.f.clone(),
        c,
    )
}

fn any_f64() -> impl Strategy<Value = f64> {
    prop_oneof![
        3 => any::<f64>(),
        1 => prop::sample::select(vec![
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            0.0,
            -0.0,
            f64::MAX,
            f64::MIN,
            f64::MIN_POSITIVE,
            1e300,
            -1e300,
        ]),
    ]
}

fn index() -> impl Strategy<Value = usize> {
    prop_oneof![4 => 0..8usize, 1 => any::<usize>()]
}

fn any_key() -> impl Strategy<Value = EdgeKey> {
    (index(), index()).prop_map(|(a, b)| EdgeKey::new(a, b))
}

fn any_cage() -> impl Strategy<Value = CageModel> {
    (
        prop::collection::vec(prop::array::uniform3(any_f64()), 0..8),
        prop::collection::vec(prop::collection::vec(index(), 0..=6), 0..6),
        prop::collection::vec(any_key(), 0..5),
    )
        .prop_map(|(v, f, c)| CageModel {
            v,
            f,
            creases: c.into_iter().collect(),
        })
}

/// 面の番号がすべて範囲内のケージ。
fn wellformed_cage() -> impl Strategy<Value = CageModel> {
    (1..8usize).prop_flat_map(|nv| {
        (
            prop::collection::vec(prop::array::uniform3(any_f64()), nv),
            prop::collection::vec(prop::collection::vec(0..nv, 0..=6), 0..6),
            prop::collection::vec((0..nv, 0..nv), 0..5),
        )
            .prop_map(|(v, f, c)| CageModel {
                v,
                f,
                creases: c.into_iter().map(|(a, b)| EdgeKey::new(a, b)).collect(),
            })
    })
}

fn any_selection() -> impl Strategy<Value = Selection> {
    prop_oneof![
        prop::collection::vec(index(), 0..6).prop_map(Selection::Vertices),
        prop::collection::vec(any_key(), 0..6).prop_map(Selection::Edges),
        prop::collection::vec(index(), 0..6).prop_map(Selection::Faces),
    ]
}

fn any_mode() -> impl Strategy<Value = SelectMode> {
    prop::sample::select(vec![SelectMode::Vertex, SelectMode::Edge, SelectMode::Face])
}

/// プリセットの範囲から要素を選んだ選択。
fn pick_selection(m: &CageModel, mode: SelectMode, picks: &[usize]) -> Selection {
    let keys: Vec<EdgeKey> = edge_map(&m.f).keys().collect();
    match mode {
        SelectMode::Vertex => Selection::Vertices(dedup(picks.iter().map(|p| p % m.v.len()))),
        SelectMode::Edge => Selection::Edges(
            dedup(picks.iter().map(|p| p % keys.len()))
                .into_iter()
                .map(|i| keys[i])
                .collect(),
        ),
        SelectMode::Face => Selection::Faces(dedup(picks.iter().map(|p| p % m.f.len()))),
    }
}

fn picks() -> impl Strategy<Value = Vec<usize>> {
    prop::collection::vec(0..1000usize, 0..=6)
}

fn sorted(mut x: Vec<usize>) -> Vec<usize> {
    x.sort_unstable();
    x
}

fn faces_of(s: &Selection) -> Vec<usize> {
    match s {
        Selection::Faces(f) => f.clone(),
        other => panic!("面の選択でない: {other:?}"),
    }
}

// ---- JS テストの移植 ----

proptest! {
    #![proptest_config(ProptestConfig::with_cases(150))]

    #[test]
    fn p01_編集操作の列のあとも位相が正しい(p in any_preset(), ops in prop::collection::vec(any_op(), 0..=8)) {
        let mut m = p.make();
        for o in &ops {
            m = apply(&m, o, o.kind);
            prop_assert_eq!(problems(&m), Vec::<String>::new());
        }
    }

    #[test]
    fn p02_編集操作の列のあとも曲面の評価値が有限(p in any_preset(), ops in prop::collection::vec(any_op(), 0..=8)) {
        let m = apply_all(p, &ops, None);
        prop_assert!(finite_mesh(&build(&m, 4)));
    }

    #[test]
    fn p03_ループ分割だけの列でg1が1度未満(p in any_preset(), ops in prop::collection::vec(any_op(), 1..=5)) {
        let m = apply_all(p, &ops, Some(1));
        let c = build(&m, 4).check;
        prop_assert!(c.g1 < 1.0, "g1={}", c.g1);
        prop_assert!(c.g0 < 1e-9, "g0={}", c.g0);
    }

    #[test]
    fn p04_面を1枚押し出すと面と頂点が面の辺数だけ増える(p in any_preset(), pick in 0..100usize, d in 0.1..=2.0f64) {
        let m = p.make();
        let fi = pick % m.f.len();
        let n = m.f[fi].len();
        let r = extrude_faces(&m, &[fi], d).unwrap();
        prop_assert_eq!(r.model.f.len(), m.f.len() + n);
        prop_assert_eq!(r.model.v.len(), m.v.len() + n);
        prop_assert_eq!(r.faces, vec![fi]);
    }

    #[test]
    fn p06_押し出し量が任意でも位相が正しく評価値が有限(p in any_preset(), d in any_f64()) {
        let r = extrude_faces(&p.make(), &[0], d).unwrap();
        prop_assert_eq!(problems(&r.model), Vec::<String>::new());
        prop_assert!(finite_mesh(&build(&r.model, 4)));
    }

    #[test]
    fn p07_有効な面番号がなければ押し出しは何も変えない(p in any_preset(), off in prop::collection::vec(0..1000usize, 0..=4)) {
        let m = p.make();
        let idx: Vec<usize> = off.iter().map(|o| m.f.len() + o).collect();
        let r = extrude_faces(&m, &idx, 0.3).unwrap();
        prop_assert!(r.faces.is_empty());
        prop_assert_eq!(r.model, m);
    }

    #[test]
    fn p08_有効と範囲外が混ざっても有効な面だけ押し出す(p in any_preset(), idx in prop::collection::vec(0..40usize, 0..=4)) {
        let m = p.make();
        let valid: HashSet<usize> = idx.iter().copied().filter(|&i| i < m.f.len()).collect();
        let r = extrude_faces(&m, &idx, 0.3).unwrap();
        prop_assert_eq!(r.faces.len(), valid.len());
        prop_assert_eq!(problems(&r.model), Vec::<String>::new());
    }

    #[test]
    fn p09_存在しない辺だけのループ分割は何も変えない(p in any_preset(), ks in prop::collection::vec((0..60usize, 0..60usize), 0..=3)) {
        let m = p.make();
        let em = edge_map(&m.f);
        let keys: Vec<EdgeKey> = ks.iter().map(|&(a, b)| EdgeKey::new(a, b)).filter(|k| !em.contains(*k)).collect();
        let cut = loop_cut(&m, &keys, None).unwrap();
        prop_assert!(cut.edges.is_empty());
        prop_assert_eq!(cut.model, m);
    }

    #[test]
    fn p10_全部ではない面の削除のあとも位相が正しい(p in any_preset(), ps in prop::collection::vec(0..100usize, 0..=5)) {
        let m = p.make();
        let nf = m.f.len();
        let idx: Vec<usize> = ps.iter().map(|x| x % nf).take(nf - 1).collect();
        let out = delete_faces(&m, &idx).unwrap();
        prop_assert_eq!(problems(&out), Vec::<String>::new());
    }

    #[test]
    fn p11_全面削除で頂点も面も空(p in any_preset()) {
        let m = p.make();
        let all: Vec<usize> = (0..m.f.len()).collect();
        let out = delete_faces(&m, &all).unwrap();
        prop_assert!(out.v.is_empty() && out.f.is_empty());
    }

    #[test]
    fn p12_変換の出力は変換先モードの範囲内の要素だけ(p in any_preset(), from in any_mode(), to in any_mode(), pk in picks()) {
        let m = p.make();
        let out = convert_selection(&m, &pick_selection(&m, from, &pk), to);
        prop_assert_eq!(out.mode(), to);
        let em = edge_map(&m.f);
        match out {
            Selection::Vertices(vs) => prop_assert!(vs.iter().all(|&v| v < m.v.len())),
            Selection::Faces(fs) => prop_assert!(fs.iter().all(|&f| f < m.f.len())),
            Selection::Edges(es) => prop_assert!(es.iter().all(|&k| em.contains(k))),
        }
    }

    #[test]
    fn p13_面の選択を点に変換すると面の頂点の和集合(p in any_preset(), pk in picks()) {
        let m = p.make();
        let sel = pick_selection(&m, SelectMode::Face, &pk);
        let want: HashSet<usize> = faces_of(&sel).iter().flat_map(|&i| m.f[i].clone()).collect();
        let Selection::Vertices(got) = convert_selection(&m, &sel, SelectMode::Vertex) else {
            return Err(TestCaseError::fail("点の選択でない"));
        };
        prop_assert_eq!(sorted(got), sorted(want.into_iter().collect()));
    }

    #[test]
    fn p14_面の選択を点か辺経由で面に戻しても元の面が残る(p in any_preset(), pk in picks()) {
        let m = p.make();
        let sel = pick_selection(&m, SelectMode::Face, &pk);
        for via in [SelectMode::Vertex, SelectMode::Edge] {
            let back = faces_of(&convert_selection(&m, &convert_selection(&m, &sel, via), SelectMode::Face));
            for f in faces_of(&sel) {
                prop_assert!(back.contains(&f), "面{}が失われた（{:?}経由）", f, via);
            }
        }
    }

    #[test]
    fn p17_不正な選択要素でもpanicせず点が範囲内(p in any_preset(), sel in any_selection(), to in any_mode()) {
        let m = p.make();
        let _ = convert_selection(&m, &sel, to);
        prop_assert!(selection_vertices(&m, &sel).iter().all(|&v| v < m.v.len()));
    }

    #[test]
    fn p18_対称な点の組でミラーが互いに指し合う(pts in prop::collection::vec((0.001..=5.0f64, -1000.0..=1000.0f64, -1000.0..=1000.0f64), 1..=8)) {
        let mut seen = HashSet::new();
        let r = |x: f64| (x * 100.0 + 0.5).floor() as i64;
        let pts: Vec<_> = pts.into_iter().filter(|&(x, y, z)| seen.insert((r(x), r(y), r(z)))).collect();
        let v: Vec<Vec3> = pts.iter().flat_map(|&(x, y, z)| [[x, y, z], [-x, y, z]]).collect();
        let mm = mirror_map(&v, DEFAULT_MIRROR_EPS);
        for (i, j) in mm.iter().enumerate() {
            let j = j.ok_or_else(|| TestCaseError::fail(format!("頂点{i}に相手がない")))?;
            prop_assert_eq!(mm[j], Some(i));
            prop_assert!((v[i][0] + v[j][0]).abs() < 1e-9);
        }
    }
}

#[test]
fn p05_立方体の上面を押し出すと頂点12面10() {
    let r = extrude_faces(&Preset::Cube.make(), &[0], 0.5)
        .unwrap()
        .model;
    assert_eq!((r.v.len(), r.f.len()), (12, 10));
}

#[test]
fn p15_立方体の面0を辺に変換すると4本() {
    let m = Preset::Cube.make();
    let Selection::Edges(es) = convert_selection(&m, &Selection::Faces(vec![0]), SelectMode::Edge)
    else {
        panic!("辺の選択でない");
    };
    assert_eq!(es.len(), 4);
}

#[test]
fn p16_面の全頂点を面に変換すると元の面を含む() {
    let m = Preset::Cube.make();
    let sel = Selection::Vertices(m.f[0].clone());
    assert!(faces_of(&convert_selection(&m, &sel, SelectMode::Face)).contains(&0));
}

#[test]
fn p19_ミラーの相手がない頂点はnone() {
    let e = DEFAULT_MIRROR_EPS;
    assert_eq!(mirror_map(&[[1.0, 0.0, 0.0]], e), vec![None]);
    assert_eq!(mirror_map(&[[0.0, 1.0, 2.0]], e), vec![Some(0)]);
    assert_eq!(mirror_map(&[], e), Vec::<Option<usize>>::new());
}

// ---- Rust 独自 ----

fn mid_map() -> impl Strategy<Value = Option<HashMap<EdgeKey, Vec3>>> {
    prop::option::of(
        prop::collection::vec((any_key(), prop::array::uniform3(any_f64())), 0..4)
            .prop_map(|l| l.into_iter().collect()),
    )
}

fn ok_or_mesh_error<T>(r: Result<T, MeshError>) -> Result<(), TestCaseError> {
    match r {
        Ok(_) => Ok(()),
        Err(MeshError::VertexOutOfRange { .. } | MeshError::UndefinedCorner { .. }) => Ok(()),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn p20_任意のケージと引数で全操作がpanicしない(
        m in any_cage(),
        idx in prop::collection::vec(index(), 0..6),
        d in any_f64(),
        keys in prop::collection::vec(any_key(), 0..5),
        mids in mid_map(),
        sel in any_selection(),
        to in any_mode(),
        eps in any_f64(),
    ) {
        ok_or_mesh_error(face_normal(&m.v, m.f.first().map_or(&[], |f| f.as_slice())))?;
        ok_or_mesh_error(compact(&m))?;
        ok_or_mesh_error(delete_faces(&m, &idx))?;
        ok_or_mesh_error(extrude_faces(&m, &idx, d))?;
        ok_or_mesh_error(loop_cut(&m, &keys, mids.as_ref()))?;
        let _ = toggle_creases(&m, &keys);
        let _ = selection_vertices(&m, &sel);
        let _ = convert_selection(&m, &sel, to);
        let _ = mirror_map(&m.v, eps);
        let _ = validate_model(&m);
        let _ = edge_map(&m.f);
    }

    #[test]
    fn p21_操作は入力のモデルを変えない(
        m in any_cage(),
        idx in prop::collection::vec(index(), 0..6),
        d in any_f64(),
        keys in prop::collection::vec(any_key(), 0..5),
        mids in mid_map(),
        sel in any_selection(),
        to in any_mode(),
    ) {
        let before = snapshot(&m);
        let _ = compact(&m);
        let _ = delete_faces(&m, &idx);
        let _ = extrude_faces(&m, &idx, d);
        let _ = loop_cut(&m, &keys, mids.as_ref());
        let _ = toggle_creases(&m, &keys);
        let _ = selection_vertices(&m, &sel);
        let _ = convert_selection(&m, &sel, to);
        let _ = validate_model(&m);
        prop_assert_eq!(snapshot(&m), before);
    }

    #[test]
    fn p22_立方体の面を押し出した複製頂点との距離は丸めた量(fi in 0..6usize, d in any_f64()) {
        let m = Preset::Cube.make();
        let want = if d.is_finite() { d.abs().clamp(0.02, 100.0) } else { 0.02 };
        let out = extrude_faces(&m, &[fi], d).unwrap().model;
        prop_assert_eq!(out.v.len(), 12);
        for (k, &orig) in m.f[fi].iter().enumerate() {
            let (a, b) = (out.v[8 + k], m.v[orig]);
            let dist = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            prop_assert!((dist - want).abs() <= 1e-9 * want.max(1.0), "角{}: 距離{} 期待{}", k, dist, want);
        }
    }

    #[test]
    fn p23_辺の折れ目を2回切り替えると元に戻る(
        p in any_preset(),
        pk in prop::collection::vec(0..1000usize, 1..=5),
        on in any::<bool>(),
        others in prop::collection::vec(0..1000usize, 0..=4),
    ) {
        let mut m = p.make();
        let keys: Vec<EdgeKey> = edge_map(&m.f).keys().collect();
        let target: Vec<EdgeKey> = pk.iter().map(|i| keys[i % keys.len()]).collect();
        // 一部だけが折れ目の状態は、2回で元に戻らないため、対象を全部入れるか全部外す
        for i in others {
            m.creases.insert(keys[i % keys.len()]);
        }
        for k in &target {
            if on { m.creases.insert(*k); } else { m.creases.remove(k); }
        }
        let twice = toggle_creases(&toggle_creases(&m, &target), &target);
        prop_assert_eq!(twice, m);
    }

    #[test]
    fn p24_compactは冪等で未使用頂点が残らない(m in wellformed_cage()) {
        let c = compact(&m).unwrap();
        let again = compact(&c.model).unwrap();
        prop_assert_eq!(snapshot(&again.model), snapshot(&c.model));
        let identity: Vec<Option<usize>> = (0..c.model.v.len()).map(Some).collect();
        prop_assert_eq!(again.remap, identity);
        let unused = validate_model(&c.model)
            .iter()
            .any(|x| matches!(x, ModelProblem::UnusedVertex { vertex: _ }));
        prop_assert!(!unused);
        let news: Vec<usize> = c.remap.iter().flatten().copied().collect();
        prop_assert!(news.windows(2).all(|w| w[0] < w[1]));
        prop_assert_eq!(news.len(), c.model.v.len());
        prop_assert_eq!(c.remap.len(), m.v.len());
    }

    #[test]
    fn p25_辺の表は面の角をすべて含む(f in prop::collection::vec(prop::collection::vec(index(), 0..=6), 0..6)) {
        let em = edge_map(&f);
        let mut corners = 0;
        for (fi, face) in f.iter().enumerate() {
            for k in 0..face.len() {
                corners += 1;
                let key = EdgeKey::new(face[k], face[(k + 1) % face.len()]);
                let e = em.lookup(key);
                prop_assert!(e.is_some_and(|e| e.faces.contains(&FaceCorner { fi, k })), "面{}の角{}がない", fi, k);
            }
        }
        prop_assert_eq!(em.entries().iter().map(|e| e.faces.len()).sum::<usize>(), corners);
        let keys: Vec<EdgeKey> = em.keys().collect();
        prop_assert_eq!(keys.iter().collect::<HashSet<_>>().len(), keys.len());
    }

    #[test]
    fn p26_ループ分割の辺は出力の辺の表にある(p in any_preset(), ops in prop::collection::vec(any_op(), 0..=5), pk in prop::collection::vec(0..1000usize, 1..=3)) {
        let m = apply_all(p, &ops, None);
        let keys: Vec<EdgeKey> = edge_map(&m.f).keys().collect();
        let picked: Vec<EdgeKey> = pk.iter().map(|i| keys[i % keys.len()]).collect();
        let cut = loop_cut(&m, &picked, None).unwrap();
        let em = edge_map(&cut.model.f);
        prop_assert!(!cut.edges.is_empty());
        for k in cut.edges {
            prop_assert!(em.contains(k), "辺{}が出力にない", k);
        }
    }

    #[test]
    fn p27_小さな不変条件(a in index(), b in index(), name in ".{0,6}") {
        prop_assert_eq!(ekey(a, b), ekey(b, a));
        let mode = SelectMode::from_js_name(&name);
        prop_assert_eq!(mode, match name.as_str() { "v" => SelectMode::Vertex, "e" => SelectMode::Edge, _ => SelectMode::Face });
    }

    #[test]
    fn p28_ミラーはepsが任意でもpanicせず長さが頂点数(v in prop::collection::vec(prop::array::uniform3(any_f64()), 0..8), eps in any_f64()) {
        prop_assert_eq!(mirror_map(&v, eps).len(), v.len());
    }
}

#[test]
fn p27_プリセットの不変条件() {
    for p in PRESETS {
        assert_eq!(Preset::from_key(p.key()), Some(p));
        assert!(problems(&p.make()).is_empty(), "{}", p.key());
    }
    assert_eq!(Preset::from_key("foo"), None);
    for (s, m) in [
        (Selection::Vertices(vec![]), SelectMode::Vertex),
        (Selection::Edges(vec![]), SelectMode::Edge),
        (Selection::Faces(vec![]), SelectMode::Face),
    ] {
        assert_eq!(s.mode(), m);
    }
    assert_eq!(SelectMode::from_js_name("v"), SelectMode::Vertex);
    assert_eq!(SelectMode::from_js_name("e"), SelectMode::Edge);
}
