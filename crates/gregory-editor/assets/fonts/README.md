# フォント

## gregory-ui-sans.ttf

Gregory UI Sans は、HackGen（白源）v2.10.0 の標準版 Regular（`HackGen-Regular.ttf`）を改変したフォントである。
元フォントの配布元は <https://github.com/yuru7/HackGen/releases/tag/v2.10.0>、作者は Yuko OTAWARA。

SIL Open Font License 1.1（OFL）に基づいて改変・再配布している。
元フォントは Reserved Font Name（RFN）「白源」「HackGen」を持つため、改変版ではこれらの名前を使わず「Gregory UI Sans」に改名した。
このフォントは上流の作者・配布元とは無関係であり、不具合は上流に問い合わせないこと。

### 改変内容

- サブセット化：`subset-chars.txt` に列挙した文字（ASCII 印字可能文字、UI 用の記号、`gregory-editor` の `src/` と `index.html`、`gregory-core` の `mesh/presets.rs`・`error.rs`・`mesh/error.rs` に含まれる非 ASCII 文字）だけを残した。レイアウト機能とヒンティングは既定のまま。
- 改名：name テーブルのうち、ファミリ名・フルネーム・PostScript 名・一意の ID など RFN を含むレコードを「Gregory UI Sans」系の名前に置き換えるか、削除した。著作権表示（nameID 0）とライセンス説明（nameID 13, 14）は元のまま残している。

### ライセンス

HackGen の zip にはライセンスファイルが含まれない。次のファイルは、上流リポジトリのタグ `v2.10.0` から取得して同ディレクトリに置いている（再生成時に sha256 を検証する）。

- `LICENSE`：HackGen 本体のライセンス（OFL 1.1。Reserved Font Name「白源」「HackGen」）
- `LICENSE_Hack`：Hack のライセンス（MIT。Hack が元にした Bitstream Vera の Reserved Font Name「Bitstream」「Vera」の記載を含む）
- `LICENSE_GenJyuuGothic`：源柔ゴシックのライセンス（OFL 1.1。Copyright (c) 2015 自家製フォント工房）

由来フォントは、HackGen の README（`HackGen is a composite font of Hack and GenJyuu-Gothic.`）に従い、Hack と源柔ゴシック（源ノ角ゴシックの派生フォント）である。
`LICENSE_GenJyuuGothic` には Reserved Font Name の記載がない。「Gregory UI Sans」はいずれの Reserved Font Name とも衝突しない。

- HackGen のリポジトリ：<https://github.com/yuru7/HackGen>
- Hack：<https://github.com/source-foundry/Hack>
- 源柔ゴシックのライセンス全文：<https://github.com/yuru7/HackGen/blob/v2.10.0/source/LICENSE_GenJyuuGothic>

### 再生成

ネットワークが必要。生成物はコミットし、CI では実行しない。

```sh
npm run font:subset
# または
bash scripts/subset-ui-font.sh
```

UI に新しい文字を追加したら再生成し、`subset-chars.txt` の差分で文字の増減を確認する。
スクリプトは元の zip の sha256 を検証する。`bash scripts/subset-ui-font.sh --print-sha256` で値を表示できる。
