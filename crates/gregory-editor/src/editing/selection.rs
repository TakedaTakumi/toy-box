//! 選択のモードと要素。クリックでの追加・解除・置換、全選択、モードの切り替え。

use gregory_core::{CageModel, EdgeKey, SelectMode, Selection, convert_selection, edge_map};

/// 選択の対象になる1つの要素。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Element {
    Vertex(usize),
    Edge(EdgeKey),
    Face(usize),
}

impl Element {
    pub fn mode(self) -> SelectMode {
        match self {
            Self::Vertex(_) => SelectMode::Vertex,
            Self::Edge(_) => SelectMode::Edge,
            Self::Face(_) => SelectMode::Face,
        }
    }

    /// ケージに実在する要素か（範囲外の番号や、面にない辺は偽）。
    pub fn exists_in(self, cage: &CageModel) -> bool {
        match self {
            Self::Vertex(v) => v < cage.v.len(),
            Self::Edge(key) => edge_map(&cage.f).contains(key),
            Self::Face(f) => f < cage.f.len(),
        }
    }
}

/// モードの表示名。
pub fn mode_label(mode: SelectMode) -> &'static str {
    match mode {
        SelectMode::Vertex => "点",
        SelectMode::Edge => "辺",
        SelectMode::Face => "面",
    }
}

/// 選択のモードと、選んだ要素（選んだ順）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditSelection {
    items: Selection,
}

impl Default for EditSelection {
    fn default() -> Self {
        Self::new(SelectMode::Vertex)
    }
}

impl EditSelection {
    pub fn new(mode: SelectMode) -> Self {
        Self { items: empty(mode) }
    }

    pub fn mode(&self) -> SelectMode {
        self.items.mode()
    }

    pub fn items(&self) -> &Selection {
        &self.items
    }

    pub fn len(&self) -> usize {
        match &self.items {
            Selection::Vertices(v) => v.len(),
            Selection::Edges(e) => e.len(),
            Selection::Faces(f) => f.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains(&self, element: Element) -> bool {
        match (&self.items, element) {
            (Selection::Vertices(v), Element::Vertex(x)) => v.contains(&x),
            (Selection::Edges(e), Element::Edge(x)) => e.contains(&x),
            (Selection::Faces(f), Element::Face(x)) => f.contains(&x),
            _ => false,
        }
    }

    /// クリックでの選択。追加なら、選択済みの要素は外し、未選択なら足す。追加でなければ、その要素だけにする。
    /// いまのモードと違う種類の要素は無視する。
    pub fn click(&mut self, element: Element, additive: bool) {
        if element.mode() != self.mode() {
            return;
        }
        let selected = self.contains(element);
        if !additive {
            self.items = empty(self.mode());
        }
        if additive && selected {
            self.remove(element);
        } else {
            self.push(element);
        }
    }

    /// 要素を押したときの選択。選択済みの要素を修飾なしで押したときは、選択を保ったまま移動に入れるよう変えない。
    pub fn press(&mut self, element: Element, additive: bool) {
        if !additive && self.contains(element) {
            return;
        }
        self.click(element, additive);
    }

    /// モードを切り替え、いまの選択を新しいモードの要素に変換する。
    pub fn set_mode(&mut self, cage: &CageModel, mode: SelectMode) {
        if mode != self.mode() {
            self.items = convert_selection(cage, &self.items, mode);
        }
    }

    pub fn select_all(&mut self, cage: &CageModel) {
        self.items = match self.mode() {
            SelectMode::Vertex => Selection::Vertices((0..cage.v.len()).collect()),
            SelectMode::Edge => Selection::Edges(edge_map(&cage.f).keys().collect()),
            SelectMode::Face => Selection::Faces((0..cage.f.len()).collect()),
        };
    }

    /// モードは変えずに、選択を空にする。
    pub fn clear(&mut self) {
        self.items = empty(self.mode());
    }

    /// 選択に含まれる頂点（重複なし、範囲外は除く）。
    pub fn vertices(&self, cage: &CageModel) -> Vec<usize> {
        gregory_core::selection_vertices(cage, &self.items)
            .into_iter()
            .filter(|&v| v < cage.v.len())
            .collect()
    }

    /// 点モードで1つだけ選んでいるときの、その頂点。
    pub fn single_vertex(&self) -> Option<usize> {
        match &self.items {
            Selection::Vertices(v) if v.len() == 1 => v.first().copied(),
            _ => None,
        }
    }

    fn push(&mut self, element: Element) {
        match (&mut self.items, element) {
            (Selection::Vertices(v), Element::Vertex(x)) => v.push(x),
            (Selection::Edges(e), Element::Edge(x)) => e.push(x),
            (Selection::Faces(f), Element::Face(x)) => f.push(x),
            _ => {}
        }
    }

    fn remove(&mut self, element: Element) {
        match (&mut self.items, element) {
            (Selection::Vertices(v), Element::Vertex(x)) => v.retain(|&y| y != x),
            (Selection::Edges(e), Element::Edge(x)) => e.retain(|&y| y != x),
            (Selection::Faces(f), Element::Face(x)) => f.retain(|&y| y != x),
            _ => {}
        }
    }
}

fn empty(mode: SelectMode) -> Selection {
    match mode {
        SelectMode::Vertex => Selection::Vertices(Vec::new()),
        SelectMode::Edge => Selection::Edges(Vec::new()),
        SelectMode::Face => Selection::Faces(Vec::new()),
    }
}
