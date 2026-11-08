# toy-box

アーティファクトとして作成したプロトタイプを格納し、GitHub Pages として公開するリポジトリです。

## 構成

- `public/` : 公開対象。`public/index.html` が一覧ページ
- `public/<プロトタイプ名>/index.html` : 各プロトタイプ(画像・JSなどは同じフォルダに置く)

## プロトタイプの追加

1. `public/<プロトタイプ名>/` を作り、`index.html` を置く
2. `public/index.html` の `<ul>` にリンクを追加する
3. main にマージすると GitHub Actions で自動公開される

## 初回設定

Settings → Pages → Source を「GitHub Actions」にする。
