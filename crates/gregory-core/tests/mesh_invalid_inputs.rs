use gregory_core::*;
use std::collections::{HashMap, HashSet};

const NAN: f64 = f64::NAN;
const INF: f64 = f64::INFINITY;

fn cube() -> CageModel {
    Preset::Cube.make()
}

fn with_face(mut m: CageModel, face: Vec<usize>) -> CageModel {
    m.f.push(face);
    m
}

fn extruded(m: &CageModel, idx: &[usize], d: f64) -> Extruded {
    extrude_faces(m, idx, d).unwrap()
}

fn key(a: usize, b: usize) -> EdgeKey {
    EdgeKey::new(a, b)
}

fn faces(s: Selection) -> Vec<usize> {
    match s {
        Selection::Faces(f) => f,
        other => panic!("面の選択でない: {other:?}"),
    }
}

// ---- 押し出し量 ----

#[test]
fn 押し出し量がnanと正の無限大なら0_02() {
    let m = cube();
    let want = extruded(&m, &[0], 0.02);
    assert_eq!(extruded(&m, &[0], NAN), want);
    assert_eq!(extruded(&m, &[0], INF), want);
}

#[test]
fn 押し出し量が負の無限大なら負の0_02で大きさは100にならない() {
    let m = cube();
    let got = extruded(&m, &[0], f64::NEG_INFINITY);
    assert_eq!(got, extruded(&m, &[0], -0.02));
    assert_ne!(got, extruded(&m, &[0], -100.0));
    assert_ne!(got, extruded(&m, &[0], 0.02));
}

#[test]
fn 押し出し量が0と負の0と小さい値は符号を保つ0_02() {
    let m = cube();
    let plus = extruded(&m, &[0], 0.02);
    for d in [0.0, -0.0, 0.019, 1e-300] {
        assert_eq!(extruded(&m, &[0], d), plus, "d={d:?}");
    }
    let minus = extruded(&m, &[0], -0.02);
    for d in [-0.019, -1e-300] {
        assert_eq!(extruded(&m, &[0], d), minus, "d={d:?}");
    }
}

#[test]
fn 押し出し量の絶対値が100超なら符号を保つ100() {
    let m = cube();
    for d in [100.5, 1e300, f64::MAX] {
        assert_eq!(extruded(&m, &[0], d), extruded(&m, &[0], 100.0), "d={d:?}");
        assert_eq!(
            extruded(&m, &[0], -d),
            extruded(&m, &[0], -100.0),
            "d={d:?}"
        );
    }
}

// ---- 押し出す面 ----

#[test]
fn 押し出す面の番号が範囲外なら無視する() {
    let m = cube();
    let r = extruded(&m, &[0, 6, usize::MAX], 0.5);
    assert_eq!(r, extruded(&m, &[0], 0.5));
}

#[test]
fn 押し出す面の番号の重複は最初の1回だけで初出の順() {
    let r = extruded(&cube(), &[2, 0, 2, 0, 3], 0.5);
    assert_eq!(r.faces, vec![2, 0, 3]);
    assert_eq!(r.model, extruded(&cube(), &[2, 0, 3], 0.5).model);
}

#[test]
fn 押し出す面が1つもなければ入力の複製で空のfaces() {
    let m = cube();
    for idx in [&[][..], &[6, 100, usize::MAX][..]] {
        let r = extruded(&m, idx, 0.5);
        assert!(r.faces.is_empty());
        assert_eq!(r.model, m);
    }
}

#[test]
fn 頂点だけを共有する選択面は頂点を面ごとに別々に複製する() {
    let m = Preset::Plane.make();
    let r = extruded(&m, &[0, 3], 0.5).model;
    assert_eq!(r.v.len(), 9 + 8);
    let adjacent = extruded(&m, &[0, 1], 0.5).model;
    assert_eq!(adjacent.v.len(), 9 + 6);
}

#[test]
fn 押し出しで選択した面が存在しない頂点を参照するとerr() {
    let m = with_face(cube(), vec![0, 1, 99]);
    assert!(matches!(
        extrude_faces(&m, &[6], 0.5),
        Err(MeshError::VertexOutOfRange { vertex: 99, .. })
    ));
}

#[test]
fn 押し出しで選択しない面の範囲外の番号は新しい頂点に重なればok重ならなければerr() {
    let overlap = with_face(cube(), vec![0, 1, 9]);
    assert!(extrude_faces(&overlap, &[0], 0.5).is_ok());
    let beyond = with_face(cube(), vec![0, 1, 99]);
    assert!(matches!(
        extrude_faces(&beyond, &[0], 0.5),
        Err(MeshError::VertexOutOfRange { vertex: 99, .. })
    ));
}

#[test]
fn 頂点が3つ未満の面と空の面と頂点が重複する面を押し出してもpanicしない() {
    for face in [vec![], vec![0], vec![0, 1], vec![0, 0, 1], vec![0, 1, 2, 2]] {
        let m = CageModel {
            v: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            f: vec![face.clone()],
            creases: HashSet::new(),
        };
        assert!(extrude_faces(&m, &[0], 0.5).is_ok(), "face={face:?}");
    }
}

// ---- 削除 ----

#[test]
fn 削除する面の番号が範囲外と重複は無視する() {
    let m = cube();
    assert_eq!(
        delete_faces(&m, &[0, 0, 6, usize::MAX]).unwrap(),
        delete_faces(&m, &[0]).unwrap()
    );
    assert_eq!(delete_faces(&m, &[]).unwrap(), compact(&m).unwrap().model);
}

#[test]
fn すべての面を削除すると頂点も面も空() {
    let m = cube();
    let all: Vec<usize> = (0..m.f.len()).collect();
    let out = delete_faces(&m, &all).unwrap();
    assert!(out.v.is_empty() && out.f.is_empty());
}

#[test]
fn 削除後に残る面が存在しない頂点を参照するとcompactもerr() {
    let m = with_face(cube(), vec![0, 1, 99]);
    assert!(matches!(
        delete_faces(&m, &[0]),
        Err(MeshError::VertexOutOfRange { vertex: 99, .. })
    ));
    assert!(matches!(
        compact(&m),
        Err(MeshError::VertexOutOfRange { vertex: 99, .. })
    ));
    assert!(delete_faces(&m, &[6]).is_ok());
}

// ---- ループ分割・折れ目 ----

#[test]
fn 存在しない辺は無視し全部無視なら入力の複製() {
    let m = cube();
    let missing = [key(0, 6), key(100, 200)];
    let cut = loop_cut(&m, &missing, None).unwrap();
    assert!(cut.edges.is_empty());
    assert_eq!(cut.model, m);
    assert_eq!(toggle_creases(&m, &missing), m);
    let with_real = loop_cut(&m, &[key(4, 5), key(0, 6)], None).unwrap();
    assert_eq!(with_real, loop_cut(&m, &[key(4, 5)], None).unwrap());
}

#[test]
fn 辺の名前は向きによらず正規形になる() {
    assert_eq!(key(1, 0), key(0, 1));
    assert_eq!(key(1, 0).to_string(), "0-1");
    let m = cube();
    assert_eq!(
        loop_cut(&m, &[key(5, 4)], None).unwrap(),
        loop_cut(&m, &[key(4, 5)], None).unwrap()
    );
}

#[test]
fn ループ分割の辺の重複は1回だけ分割する() {
    let m = cube();
    assert_eq!(
        loop_cut(&m, &[key(4, 5), key(5, 4), key(4, 5)], None).unwrap(),
        loop_cut(&m, &[key(4, 5)], None).unwrap()
    );
}

#[test]
fn 折れ目の辺の重複は結果を変えない() {
    let m = cube();
    let (a, b) = (key(4, 5), key(5, 6));
    assert_eq!(toggle_creases(&m, &[a, a, b]), toggle_creases(&m, &[a, b]));
    let on = toggle_creases(&m, &[a, b]);
    assert!(on.creases.contains(&a) && on.creases.contains(&b));
    // すべてが折れ目なら外し、そうでなければすべてを付ける
    assert!(toggle_creases(&on, &[a, b, a]).creases.is_empty());
    let partial = toggle_creases(&m, &[a]);
    assert_eq!(toggle_creases(&partial, &[a, b]).creases.len(), 2);
}

#[test]
fn ループ分割の辺の端点が存在しない頂点ならmid_ofがなければerrあれば通る() {
    let m = CageModel {
        v: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
        f: vec![vec![0, 1, 5, 2]],
        creases: HashSet::new(),
    };
    let k = key(1, 5);
    assert!(matches!(
        loop_cut(&m, &[k], None),
        Err(MeshError::VertexOutOfRange { vertex: 5, .. })
    ));
    let mids = HashMap::from([(k, [1.0, 0.5, 0.0]), (key(0, 2), [0.0, 0.5, 0.0])]);
    assert!(loop_cut(&m, &[k], Some(&mids)).is_ok());
}

#[test]
fn ループ分割で頂点が1から2個の面はundefined_cornerのerr() {
    for face in [vec![0], vec![0, 1]] {
        let m = CageModel {
            v: vec![[0., 0., 0.], [1., 0., 0.]],
            f: vec![face.clone()],
            creases: HashSet::new(),
        };
        let k = key(face[0], face[face.len() - 1]);
        assert!(
            matches!(
                loop_cut(&m, &[k], None),
                Err(MeshError::UndefinedCorner { .. })
            ),
            "face={face:?}"
        );
    }
}

#[test]
fn ループ分割で5辺以上の面は3角として分割するか未定義角のerr() {
    let m = CageModel {
        v: (0..5).map(|i| [f64::from(i), 0.0, 0.0]).collect(),
        f: vec![vec![0, 1, 2, 3, 4]],
        creases: HashSet::new(),
    };
    let r = loop_cut(&m, &[key(0, 1)], None);
    assert!(matches!(r, Ok(_) | Err(MeshError::UndefinedCorner { .. })));
}

#[test]
fn mid_ofの分割しない辺の位置は無視する() {
    let m = cube();
    let base = loop_cut(&m, &[key(4, 5)], None).unwrap();
    let mids = HashMap::from([(key(0, 3), [9.0, 9.0, 9.0]), (key(100, 101), [1.0; 3])]);
    assert_eq!(loop_cut(&m, &[key(4, 5)], Some(&mids)).unwrap(), base);
}

// ---- 座標 ----

#[test]
fn 座標がnanや無限大でも例外なしで値が伝わる() {
    let mut m = cube();
    m.v[0] = [NAN, INF, f64::NEG_INFINITY];
    assert!(extrude_faces(&m, &[0], 0.5).is_ok());
    assert!(loop_cut(&m, &[key(4, 5)], None).is_ok());
    assert!(delete_faces(&m, &[1]).is_ok());
    assert!(compact(&m).is_ok());
    let problems = validate_model(&m);
    assert!(problems.contains(&ModelProblem::InvalidCoordinate { vertex: 0 }));
    let r = extruded(&m, &[2], 0.5).model;
    assert!(r.v.iter().any(|p| p.iter().any(|x| !x.is_finite())));
}

// ---- 選択 ----

#[test]
fn 選択のモードが_v_e_以外なら面のモード() {
    for name in ["f", "", "V", "E", "x", "vertex", "ve"] {
        assert_eq!(SelectMode::from_js_name(name), SelectMode::Face, "{name:?}");
    }
    assert_eq!(SelectMode::from_js_name("v"), SelectMode::Vertex);
    assert_eq!(SelectMode::from_js_name("e"), SelectMode::Edge);
}

#[test]
fn 選択の点と面の番号が範囲外なら無視する() {
    let m = cube();
    assert_eq!(
        selection_vertices(&m, &Selection::Vertices(vec![1, 8, usize::MAX, 1, 2])),
        vec![1, 2]
    );
    assert_eq!(
        selection_vertices(&m, &Selection::Faces(vec![6, usize::MAX])),
        Vec::<usize>::new()
    );
    let out = convert_selection(
        &m,
        &Selection::Vertices(vec![100, usize::MAX]),
        SelectMode::Face,
    );
    assert!(faces(out).is_empty());
}

#[test]
fn 選択の辺の端点が範囲外なら無視する() {
    let m = cube();
    let sel = Selection::Edges(vec![key(2, 99), key(usize::MAX, 0), key(0, 1)]);
    assert_eq!(selection_vertices(&m, &sel), vec![0, 1]);
}

#[test]
fn 選択の辺が存在しなくても両端が範囲内なら両端の頂点を入れる() {
    let m = cube();
    assert_eq!(
        selection_vertices(&m, &Selection::Edges(vec![key(0, 6)])),
        vec![0, 6]
    );
}

#[test]
fn 選択した面が存在しない頂点を参照すると範囲外の番号をそのまま返す() {
    let m = with_face(cube(), vec![0, 1, 99]);
    assert_eq!(
        selection_vertices(&m, &Selection::Faces(vec![6])),
        vec![0, 1, 99]
    );
}

#[test]
fn 面への変換で頂点のない面は常に選ばれる() {
    let m = with_face(cube(), vec![]);
    let out = convert_selection(&m, &Selection::Vertices(vec![]), SelectMode::Face);
    assert_eq!(faces(out), vec![6]);
}

// ---- ミラー ----

#[test]
fn ミラーのepsが0でも負でも無限大でもnanでもpanicしない() {
    let pair = [[1.0, 2.0, 3.0], [-1.0, 2.0, 3.0]];
    for eps in [0.0, -0.0, -1.0, INF, f64::NEG_INFINITY, NAN] {
        assert_eq!(mirror_map(&pair, eps).len(), 2, "eps={eps:?}");
    }
    assert_eq!(mirror_map(&pair, 0.0), vec![Some(1), Some(0)]);
    assert_eq!(mirror_map(&pair, -1.0), vec![Some(1), Some(0)]);
    // 丸めた位置がすべて同じになり、番号の最も大きい頂点が相手になる
    assert_eq!(mirror_map(&pair, INF), vec![Some(1), Some(1)]);
    assert_eq!(mirror_map(&pair, NAN), vec![Some(1), Some(1)]);
}

#[test]
fn ミラーで同じ位置に複数の頂点があれば番号の最も大きい頂点を相手にする() {
    let v = [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]];
    assert_eq!(
        mirror_map(&v, DEFAULT_MIRROR_EPS),
        vec![Some(2), Some(0), Some(0)]
    );
}

#[test]
fn ミラーで座標がnanならnanどうしは同じ位置とみなす() {
    let v = [[NAN, 0.0, 0.0], [NAN, 0.0, 0.0]];
    assert_eq!(mirror_map(&v, DEFAULT_MIRROR_EPS), vec![Some(1), Some(1)]);
}

// ---- プリセット ----

#[test]
fn プリセットの未知のキーはnone() {
    for k in ["foo", "", "Cube", "cube ", "__proto__", "constructor"] {
        assert_eq!(Preset::from_key(k), None, "{k:?}");
    }
}
