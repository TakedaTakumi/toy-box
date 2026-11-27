#!/usr/bin/env bash
# HackGen (白源) Regular から UI 用サブセット「Gregory UI Sans」を生成する。
# 使い方: bash scripts/subset-ui-font.sh [--print-sha256]
set -euo pipefail

HACKGEN_VERSION="v2.10.0"
HACKGEN_URL="https://github.com/yuru7/HackGen/releases/download/${HACKGEN_VERSION}/HackGen_${HACKGEN_VERSION}.zip"
# 未設定のままでは実行できない。`--print-sha256` で取得した値を、配布元と照合したうえで設定する。
HACKGEN_ZIP_SHA256="ed182e2a4b95792d94dea7932f6b45280b5ae353651be249d5f6b7867b788db7"
FONTTOOLS_VERSION="4.60.1"
# zip にライセンスが含まれないため、同じタグのリポジトリから取得する（ファイル名:sha256）。
LICENSE_BASE_URL="https://raw.githubusercontent.com/yuru7/HackGen/${HACKGEN_VERSION}"
LICENSE_FILES=(
  "LICENSE:9bfc3b3826a9c71042b87fda90a19cda4918894cdbd449cab163b7a7c93c5aa7:LICENSE"
  "source/LICENSE_Hack:1f61bb7c790c59b4b0ecdf304628b94e42ae4c8020094a8c3da73381ab212623:LICENSE_Hack"
  "source/LICENSE_GenJyuuGothic:68b83948ea44760eaf1cefc59f0ce2a381bb0e307508e8f0e5e8df529ee04170:LICENSE_GenJyuuGothic"
)
NEW_FAMILY="Gregory UI Sans"

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATE_DIR="${ROOT_DIR}/crates/gregory-editor"
OUT_DIR="${CRATE_DIR}/assets/fonts"
OUT_FONT="${OUT_DIR}/gregory-ui-sans.ttf"
CHARS_FILE="${OUT_DIR}/subset-chars.txt"

usage() {
  echo "usage: bash scripts/subset-ui-font.sh [--print-sha256]" >&2
}

PRINT_SHA=0
case "$#" in
  0) ;;
  1)
    case "$1" in
      --print-sha256) PRINT_SHA=1 ;;
      -h | --help) usage; exit 0 ;;
      *) echo "error: unknown argument: $1" >&2; usage; exit 2 ;;
    esac
    ;;
  *) echo "error: too many arguments" >&2; usage; exit 2 ;;
esac

for cmd in curl sha256sum python3; do
  command -v "$cmd" >/dev/null || { echo "error: $cmd is required" >&2; exit 1; }
done

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

ZIP="${WORK}/hackgen.zip"
echo "downloading ${HACKGEN_URL}"
curl -fsSL --retry 3 -o "$ZIP" "$HACKGEN_URL"
ACTUAL_SHA="$(sha256sum "$ZIP" | cut -d' ' -f1)"

if [ "$PRINT_SHA" -eq 1 ]; then
  echo "$ACTUAL_SHA"
  exit 0
fi

if [ -z "$HACKGEN_ZIP_SHA256" ]; then
  echo "error: HACKGEN_ZIP_SHA256 is not set (actual: ${ACTUAL_SHA}); run with --print-sha256 and verify it" >&2
  exit 1
fi
if [ "$ACTUAL_SHA" != "$HACKGEN_ZIP_SHA256" ]; then
  echo "error: sha256 mismatch (expected ${HACKGEN_ZIP_SHA256}, actual ${ACTUAL_SHA})" >&2
  exit 1
fi

[ -d "${CRATE_DIR}/src" ] || { echo "error: ${CRATE_DIR}/src not found" >&2; exit 1; }
mkdir -p "$OUT_DIR"

echo "installing fonttools==${FONTTOOLS_VERSION}"
python3 -m venv "${WORK}/venv"
"${WORK}/venv/bin/pip" install --quiet --disable-pip-version-check "fonttools==${FONTTOOLS_VERSION}"
PY="${WORK}/venv/bin/python"

# zip から元フォントを取り出す
SRC_DIR="${WORK}/src"
mkdir -p "$SRC_DIR"
"$PY" -I - "$ZIP" "$SRC_DIR" <<'PYEOF'
import os, shutil, sys, zipfile

zip_path, src_dir = sys.argv[1:3]
with zipfile.ZipFile(zip_path) as z:
    for info in z.infolist():
        if not info.is_dir() and os.path.basename(info.filename) == "HackGen-Regular.ttf":
            with z.open(info) as s, open(os.path.join(src_dir, "HackGen-Regular.ttf"), "wb") as d:
                shutil.copyfileobj(s, d)
            break
    else:
        sys.exit("error: HackGen-Regular.ttf not found in zip")
PYEOF

# ライセンス全文を取得して sha256 を検証し、出力先に置く
for entry in "${LICENSE_FILES[@]}"; do
  IFS=: read -r rel sha dest <<<"$entry"
  curl -fsSL --retry 3 -o "${WORK}/${dest}" "${LICENSE_BASE_URL}/${rel}"
  actual="$(sha256sum "${WORK}/${dest}" | cut -d' ' -f1)"
  [ "$actual" = "$sha" ] || { echo "error: sha256 mismatch for ${rel} (expected ${sha}, actual ${actual})" >&2; exit 1; }
  cp "${WORK}/${dest}" "${OUT_DIR}/${dest}"
  echo "copied license file: ${rel} -> ${OUT_DIR}/${dest}"
done

# 使う文字の一覧を作る（ASCII + 記号 + ソース中の非 ASCII）
"$PY" -I - "$CRATE_DIR" "$CHARS_FILE" "${ROOT_DIR}/crates/gregory-core/src" <<'PYEOF'
import sys, unicodedata
from pathlib import Path

crate, out, core_src = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
chars = {chr(c) for c in range(0x20, 0x7F)}
chars |= set("・、。「」『』（）［］〜ー…→←↑↓°±×÷✓▶▼　")
files = sorted((crate / "src").rglob("*.rs")) + [crate / "index.html"]
# UI に表示される文字列の出どころ
files += [core_src / "mesh" / "presets.rs", core_src / "error.rs", core_src / "mesh" / "error.rs"]
for f in files:
    if not f.is_file():
        if f.name == "presets.rs" or f == core_src / "error.rs":
            sys.exit(f"error: {f} not found")
        continue
    for ch in f.read_text(encoding="utf-8"):
        if ord(ch) > 0x7F:
            chars.add(ch)
chars = {c for c in chars if unicodedata.category(c) not in ("Cc", "Cs", "Co", "Cn", "Cf")}
lines = ["# サブセットに含める文字（scripts/subset-ui-font.sh が生成）。形式: U+XXXX<TAB>文字"]
lines += [f"U+{ord(c):04X}\t{c}" for c in sorted(chars)]
out.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"wrote {out} ({len(chars)} chars)")
PYEOF

# 一覧から pyftsubset 用のテキストを作る
TEXT_FILE="${WORK}/chars.txt"
"$PY" -I - "$CHARS_FILE" "$TEXT_FILE" <<'PYEOF'
import sys
cps = []
for line in open(sys.argv[1], encoding="utf-8"):
    if line.startswith("U+"):
        cps.append(int(line.split("\t", 1)[0][2:], 16))
if not cps:
    sys.exit("error: no characters")
open(sys.argv[2], "w", encoding="utf-8").write("".join(map(chr, cps)))
PYEOF

echo "subsetting"
"${WORK}/venv/bin/pyftsubset" "${SRC_DIR}/HackGen-Regular.ttf" \
  --text-file="$TEXT_FILE" \
  --layout-features='*' \
  --name-IDs='*' \
  --name-languages='*' \
  --output-file="${WORK}/subset.ttf"

# name テーブルを改名し、RFN が残っていないか検査する
"$PY" -I - "${SRC_DIR}/HackGen-Regular.ttf" "${WORK}/subset.ttf" "$OUT_FONT" "$CHARS_FILE" "$NEW_FAMILY" "$HACKGEN_VERSION" <<'PYEOF'
import sys
from fontTools.ttLib import TTFont

src_path, sub_path, out_path, chars_path, family, version = sys.argv[1:7]
PRESERVE = {0, 13, 14}  # 著作権表示とライセンス説明は OFL の条件として元のまま残す
RFN = ("HackGen", "白源")
ps = family.replace(" ", "") + "-Regular"
fixed = {
    1: family,
    3: f"{ps}-{version.lstrip('v')}",
    4: f"{family} Regular",
    5: f"Version {version.lstrip('v')}; {family} subset",
    6: ps,
    16: family,
    17: "Regular",
    21: family,
    22: "Regular",
}

def has_rfn(s):
    return any(k.lower() in s.lower() for k in RFN)

font = TTFont(sub_path, recalcTimestamp=False)
font["head"].modified = font["head"].created  # 実行ごとに出力が変わらないようにする
name = font["name"]
kept = []
for rec in name.names:
    if rec.nameID in PRESERVE:
        kept.append(rec)
        continue
    text = rec.toUnicode()
    if rec.nameID in fixed:
        rec.string = fixed[rec.nameID].encode(rec.getEncoding(), errors="strict")
        kept.append(rec)
    elif has_rfn(text):
        continue  # 置換しても意味を成さない（URL・説明文など）ため削除する
    else:
        kept.append(rec)
name.names = kept
for nid in (1, 3, 4, 5, 6):
    name.setName(fixed[nid], nid, 3, 1, 0x409)
    name.setName(fixed[nid], nid, 1, 0, 0)

bad = [(r.nameID, r.toUnicode()) for r in name.names
       if r.nameID not in PRESERVE and has_rfn(r.toUnicode())]
if bad:
    sys.exit(f"error: reserved font name remains in name table: {bad}")
font.save(out_path)

# 保存後のファイルでも再検査する
chk = TTFont(out_path)
bad = [(r.nameID, r.toUnicode()) for r in chk["name"].names
       if r.nameID not in PRESERVE and has_rfn(r.toUnicode())]
if bad:
    sys.exit(f"error: reserved font name remains in saved font: {bad}")
if not any(r.nameID == 0 and "Yuko OTAWARA" in r.toUnicode() for r in chk["name"].names):
    sys.exit("error: copyright (nameID 0) was lost")

# 元フォントに無い文字は警告する
cmap = TTFont(src_path).getBestCmap()
missing = []
for line in open(chars_path, encoding="utf-8"):
    if line.startswith("U+"):
        cp = int(line.split("\t", 1)[0][2:], 16)
        if cp not in cmap:
            missing.append(f"U+{cp:04X}")
if missing:
    print("warning: not in HackGen:", " ".join(missing))
print("name table OK")
PYEOF

echo "done: ${OUT_FONT} ($(wc -c < "$OUT_FONT") bytes)"
