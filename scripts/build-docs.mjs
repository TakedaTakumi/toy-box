import { readdir, readFile, writeFile } from 'node:fs/promises';
import { join, relative, sep } from 'node:path';
import { marked } from 'marked';

const ROOT = new URL('../public/', import.meta.url).pathname;

const escapeHtml = (s) =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

async function* findReadmes(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* findReadmes(path);
    else if (entry.name === 'README.md') yield path;
  }
}

function splitFrontMatter(text) {
  const m = text.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?/);
  if (!m) return { meta: {}, body: text };
  const meta = {};
  for (const line of m[1].split(/\r?\n/)) {
    const kv = line.match(/^(\w+):\s*(.*)$/);
    if (kv) meta[kv[1]] = kv[2].trim();
  }
  return { meta, body: text.slice(m[0].length) };
}

const STYLE = `
  :root { color-scheme: light dark; --bg:#f4f6fb; --surface:#fff; --ink:#1c2438; --muted:#5d6a85; --line:#dde3ef; --accent:#4f6bed; }
  @media (prefers-color-scheme: dark) {
    :root { --bg:#11151f; --surface:#1a2030; --ink:#e8ecf6; --muted:#9aa6c0; --line:#2b3550; --accent:#8da2ff; }
  }
  * { box-sizing: border-box; }
  body { margin:0; background:var(--bg); color:var(--ink); line-height:1.8;
    font-family: system-ui, -apple-system, "Hiragino Sans", "Noto Sans JP", "Yu Gothic", sans-serif; }
  main { max-width: 52rem; margin: 0 auto; padding: 32px 16px 64px; }
  nav a { color: var(--accent); }
  article { margin-top: 16px; padding: 8px 24px 24px; background: var(--surface); border: 1px solid var(--line); border-radius: 14px; overflow-wrap: anywhere; }
  h1 { font-size: 1.6rem; }
  h2 { margin-top: 2rem; padding-left: 12px; border-left: 4px solid var(--accent); font-size: 1.25rem; }
  a { color: var(--accent); }
  code { padding: 1px 5px; border-radius: 4px; background: var(--bg); font-size: .9em; }
  pre { padding: 12px 16px; border-radius: 8px; background: var(--bg); overflow-x: auto; }
  pre code { padding: 0; background: none; }
  .table-wrap { overflow-x: auto; }
  table { border-collapse: collapse; }
  th, td { padding: 6px 10px; border: 1px solid var(--line); text-align: left; vertical-align: top; }
  th { background: var(--bg); }
`;

function render({ title, body, depth }) {
  const home = '../'.repeat(depth);
  return `<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<style>${STYLE}</style>
</head>
<body>
<main>
<nav><a href="${home}">&larr; toy-box 一覧へ</a> / <a href="./">プロトタイプを開く</a></nav>
<article>
${body}
</article>
</main>
</body>
</html>
`;
}

const renderer = new marked.Renderer();
renderer.table = (header, body) =>
  `<div class="table-wrap"><table><thead>${header}</thead><tbody>${body}</tbody></table></div>`;

let count = 0;
for await (const path of findReadmes(ROOT)) {
  const { meta, body } = splitFrontMatter(await readFile(path, 'utf8'));
  const html = marked.parse(body, { renderer, gfm: true });
  const h1 = body.match(/^#\s+(.+)$/m);
  const title = meta.title || (h1 ? h1[1] : 'ドキュメント');
  const depth = relative(ROOT, path).split(sep).length - 1;
  await writeFile(path.replace(/README\.md$/, 'README.html'), render({ title, body: html, depth }));
  count += 1;
}
console.log(`${count} 件のドキュメントを変換しました`);
