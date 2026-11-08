# toy-box

アーティファクトとして作成したプロトタイプを格納し、GitHub Pages として公開するリポジトリです。

## 公開URL

- 一覧ページ: https://takedatakumi.github.io/toy-box/
- すうじモンスター: https://takedatakumi.github.io/toy-box/games/suuji-monster/
- ポリリズム／ポリメーター: https://takedatakumi.github.io/toy-box/tools/polyrhythm/

## 構成

- `public/` : 公開対象。`public/index.html` が一覧ページ
- `public/<カテゴリ>/<プロトタイプ名>/index.html` : 各プロトタイプ(画像・JS・READMEなどは同じフォルダに置く)
- カテゴリ: `games`(ゲーム) / `tools`(可視化・学習ツール)

## プロトタイプの追加

1. `public/<カテゴリ>/<プロトタイプ名>/` を作り、`index.html` を置く
2. `public/index.html` のカテゴリの見出しの下にリンクを追加する
3. この README の公開URLに追記する
4. main にマージすると GitHub Actions で自動公開される

## 初回設定

Settings → Pages → Source を「GitHub Actions」にする。
