// A small, safe Markdown renderer for the assistant's answers.
// It builds DOM nodes with textContent only, so nothing the model writes can become markup or script.
// Supported: paragraphs, "-"/"*"/"1." lists, # headings (shown bold), > quotes, --- rules, tables,
// **bold**, *italic*, ~~strike~~, `code`, ``` code blocks (with the language kept in data-lang), and
// [text](https://link) (only http and https; opened through onLink).

const LINK = /\[([^\]\n]{1,200})\]\((https?:\/\/[^\s)]{1,1000})\)/;

function inline(parent, text, onLink) {
  // Split on the first inline token and recurse on the rest.
  let rest = text;
  while (rest.length) {
    const candidates = [];
    const b = rest.indexOf('**');
    if (b >= 0) { const e = rest.indexOf('**', b + 2); if (e > b + 2) candidates.push({ at: b, kind: 'b', end: e + 2, inner: rest.slice(b + 2, e) }); }
    const st = rest.indexOf('~~');
    if (st >= 0) { const e = rest.indexOf('~~', st + 2); if (e > st + 2) candidates.push({ at: st, kind: 's', end: e + 2, inner: rest.slice(st + 2, e) }); }
    const c = rest.indexOf('`');
    if (c >= 0) { const e = rest.indexOf('`', c + 1); if (e > c + 1) candidates.push({ at: c, kind: 'code', end: e + 1, inner: rest.slice(c + 1, e) }); }
    const l = LINK.exec(rest);
    if (l) candidates.push({ at: l.index, kind: 'a', end: l.index + l[0].length, inner: l[1], href: l[2] });
    const i = rest.search(/(^|[^*\w])\*[^*\s][^*\n]*\*(?!\*)/);
    if (i >= 0) {
      const start = rest.indexOf('*', i);
      const e = rest.indexOf('*', start + 1);
      if (e > start + 1) candidates.push({ at: start, kind: 'i', end: e + 1, inner: rest.slice(start + 1, e) });
    }
    if (!candidates.length) { parent.append(document.createTextNode(rest)); return; }
    candidates.sort((x, y) => x.at - y.at);
    const t = candidates[0];
    if (t.at > 0) parent.append(document.createTextNode(rest.slice(0, t.at)));
    if (t.kind === 'b') { const e = document.createElement('strong'); inline(e, t.inner, onLink); parent.append(e); }
    else if (t.kind === 'i') { const e = document.createElement('em'); inline(e, t.inner, onLink); parent.append(e); }
    else if (t.kind === 's') { const e = document.createElement('s'); inline(e, t.inner, onLink); parent.append(e); }
    else if (t.kind === 'code') { const e = document.createElement('code'); e.textContent = t.inner; parent.append(e); }
    else {
      const e = document.createElement('a');
      e.textContent = t.inner;
      e.href = t.href;
      e.rel = 'noopener noreferrer';
      e.addEventListener('click', (ev) => { ev.preventDefault(); if (onLink) onLink(t.href); });
      parent.append(e);
    }
    rest = rest.slice(t.end);
  }
}

const isTableRow = (l) => /^\s*\|.*\|\s*$/.test(l);
const isTableRule = (l) => /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/.test(l) && l.includes('-');
const cells = (l) => l.trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map((c) => c.trim());

function renderTable(container, rows, onLink) {
  const table = document.createElement('table');
  const head = cells(rows[0]);
  const aligns = cells(rows[1]).map((c) => (c.startsWith(':') && c.endsWith(':') ? 'center' : c.endsWith(':') ? 'right' : 'left'));
  const thead = document.createElement('thead');
  const hr = document.createElement('tr');
  head.forEach((h, i) => { const th = document.createElement('th'); th.style.textAlign = aligns[i] || 'left'; inline(th, h, onLink); hr.append(th); });
  thead.append(hr);
  const tbody = document.createElement('tbody');
  rows.slice(2).forEach((r) => {
    const tr = document.createElement('tr');
    cells(r).forEach((c, i) => { const td = document.createElement('td'); td.style.textAlign = aligns[i] || 'left'; inline(td, c, onLink); tr.append(td); });
    tbody.append(tr);
  });
  table.append(thead, tbody);
  const wrap = document.createElement('div');
  wrap.className = 'md-table';
  wrap.append(table);
  container.append(wrap);
}

/** Fills `container` with the rendered text (replacing what was there). */
export function renderMarkdown(container, text, onLink) {
  container.replaceChildren();
  const lines = String(text ?? '').replace(/\r\n?/g, '\n').split('\n');
  let para = null;
  let list = null;
  let code = null;
  let quote = null;
  const flush = () => { para = null; list = null; quote = null; };
  for (let n = 0; n < lines.length; n++) {
    const raw = lines[n];
    if (code) {
      if (raw.trim().startsWith('```')) { code = null; } else { code.textContent += (code.textContent ? '\n' : '') + raw; }
      continue;
    }
    if (raw.trim().startsWith('```')) {
      flush();
      const pre = document.createElement('pre');
      const lang = raw.trim().slice(3).trim().split(/\s/)[0].slice(0, 20);
      if (lang) pre.dataset.lang = lang;
      code = document.createElement('code');
      pre.append(code);
      container.append(pre);
      continue;
    }
    const line = raw.trimEnd();
    if (!line.trim()) { flush(); continue; }
    // a table: a row of cells, then a rule, then more rows
    if (isTableRow(line) && n + 1 < lines.length && isTableRule(lines[n + 1])) {
      flush();
      const rows = [line, lines[n + 1]];
      let k = n + 2;
      while (k < lines.length && isTableRow(lines[k])) { rows.push(lines[k]); k++; }
      renderTable(container, rows, onLink);
      n = k - 1;
      continue;
    }
    let m;
    if (/^\s*([-*_])(\s*\1){2,}\s*$/.test(line)) {
      flush();
      container.append(document.createElement('hr'));
    } else if ((m = /^\s*>\s?(.*)$/.exec(line))) {
      list = null; para = null;
      if (!quote) { quote = document.createElement('blockquote'); container.append(quote); } else { quote.append(document.createElement('br')); }
      inline(quote, m[1], onLink);
    } else if ((m = /^\s*(#{1,4})\s+(.*)$/.exec(line))) {
      flush();
      const h = document.createElement('p');
      h.className = 'md-h';
      inline(h, m[2], onLink);
      container.append(h);
    } else if ((m = /^\s*(?:[-*•]|\d+[.)])\s+(.*)$/.exec(line))) {
      quote = null;
      const ordered = /^\s*\d/.test(line);
      if (!list || list.dataset.kind !== (ordered ? 'ol' : 'ul')) {
        para = null;
        list = document.createElement(ordered ? 'ol' : 'ul');
        list.dataset.kind = ordered ? 'ol' : 'ul';
        container.append(list);
      }
      const li = document.createElement('li');
      inline(li, m[1], onLink);
      list.append(li);
    } else {
      list = null; quote = null;
      if (!para) { para = document.createElement('p'); container.append(para); } else { para.append(document.createElement('br')); }
      inline(para, line.trim(), onLink);
    }
  }
}
