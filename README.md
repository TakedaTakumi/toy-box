# toy-box

アーティファクトとして作成したプロトタイプを格納し、GitHub Pages として公開するリポジトリです。

## 公開URL

- https://takedatakumi.github.io/toy-box/

## 構成

- `public/` : 公開対象。`public/index.html` が一覧ページ
- `public/<カテゴリ>/<プロトタイプ名>/index.html` : 各プロトタイプ(画像・JS・READMEなどは同じフォルダに置く)
- `public/<カテゴリ>/<プロトタイプ名>/README.md` : 各プロトタイプのドキュメント。デプロイ時に `README.html` へ変換して公開する
- `scripts/build-docs.mjs` : README.md を HTML に変換するスクリプト(`npm ci && npm run build:docs` でローカルでも確認できる。生成物はコミットしない)
- カテゴリ: `games`(ゲーム) / `tools`(可視化・学習ツール) / `art`(アート・演出)

## プロトタイプの追加

1. `public/<カテゴリ>/<プロトタイプ名>/` を作り、`index.html` を置く
2. `public/index.html` のカテゴリの見出しの下にカードを追加する(プロトタイプ本体と `README.html` の2リンク)
3. main にマージすると GitHub Actions で自動公開される

## 初回設定

Settings → Pages → Source を「GitHub Actions」にする。
