// The assistant page (odyssey://chat): a proper chat with your assistant.
//   - history on the left (pinned, then by day), search, rename, pin, delete
//   - header with the chat title, the model in use, what the assistant may do, export and delete
//   - messages: your pictures, copy / edit on your messages, copy / ask again on answers, steps the
//     assistant took (collapsed), sources, code blocks with a Copy button, tables
//   - composer: attach pictures (button, paste, drag and drop), screenshot of the page you were on
//     (visible part or the whole page), "this page" as reference, Enter to send, Stop while it works
// The DOM is built once and updated in place (chatUpdate), so streaming an answer never loses what
// you are typing or where you scrolled.

import { renderMarkdown } from './markdown.js';
import { thinkingStars } from './helm.js';

const SVG = 'http://www.w3.org/2000/svg';
const STAR = 'M12 2C12.8 8.5 15.5 11.2 22 12C15.5 12.8 12.8 15.5 12 22C11.2 15.5 8.5 12.8 2 12C8.5 11.2 11.2 8.5 12 2Z';
const ICON = {
  plus: 'M12 5v14M5 12h14',
  search: 'M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4',
  pencil: 'M4 20h4L19 9l-4-4L4 16v4z',
  copy: 'M9 9h10v11H9zM5 15V4h10',
  refresh: 'M20 12a8 8 0 1 1-2.3-5.6M20 4v5h-5',
  pin: 'M9 4h6l-1 6 3 3H7l3-3-1-6zM12 13v7',
  trash: 'M5 7h14M9 7V4h6v3M7 7l1 13h8l1-13',
  clip: 'M8 12l6-6a3 3 0 0 1 4 4l-8 8a5 5 0 0 1-7-7l7-7',
  camera: 'M4 8h4l2-3h4l2 3h4v11H4zM12 17a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7z',
  page: 'M6 3h9l4 4v14H6zM14 3v5h5',
  sidebar: 'M9 4v16M3 4h18v16H3z',
  chevron: 'M7 10l5 5 5-5',
  check: 'M5 12l5 5 10-11',
  more: 'M5 12h.01M12 12h.01M19 12h.01',
  download: 'M12 4v12m0 0l-4-4m4 4l4-4M5 20h14',
  x: 'M6 6l12 12M18 6L6 18',
  up: 'M12 19V6m0 0l-5 5m5-5l5 5',
  down: 'M12 5v13m0 0l-5-5m5 5l5-5',
  stop: 'M7 7h10v10H7z',
  tools: 'M14 6l4 4-8 8H6v-4zM13 7l4 4',
  steps: 'M5 6h14M5 12h14M5 18h9',
};

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

function icon(name, size = 16, fill = false) {
  const s = document.createElementNS(SVG, 'svg');
  for (const [k, v] of Object.entries({ width: size, height: size, viewBox: '0 0 24 24', 'aria-hidden': 'true', fill: fill ? 'currentColor' : 'none', stroke: fill ? 'none' : 'currentColor', 'stroke-width': '1.8', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' })) s.setAttribute(k, v);
  const p = document.createElementNS(SVG, 'path');
  p.setAttribute('d', ICON[name] || name);
  s.append(p);
  return s;
}

function iconButton(name, label, onClick, cls = '') {
  const b = el('button', `chat-ib ${cls}`.trim());
  b.type = 'button';
  b.setAttribute('aria-label', label);
  b.title = label;
  b.append(icon(name, 15));
  b.addEventListener('click', (e) => { e.stopPropagation(); onClick(e); });
  return b;
}

/** "just now", "12 min ago", "Yesterday", or a short date. */
export function relativeTime(seconds, now = Date.now() / 1000) {
  const d = Math.max(0, now - seconds);
  if (d < 60) return 'just now';
  if (d < 3600) return `${Math.floor(d / 60)} min ago`;
  if (d < 86400) return `${Math.floor(d / 3600)} h ago`;
  if (d < 172800) return 'Yesterday';
  return new Date(seconds * 1000).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' });
}

/** The heading a conversation is listed under. */
export function dayGroup(seconds, pinned, now = Date.now() / 1000) {
  if (pinned) return 'Pinned';
  const d = now - seconds;
  if (d < 86400) return 'Today';
  if (d < 172800) return 'Yesterday';
  if (d < 7 * 86400) return 'Previous 7 days';
  return 'Earlier';
}

function clock(seconds) {
  return seconds ? new Date(seconds * 1000).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }) : '';
}

/** Copies text; true when it worked. */
export async function copyText(text) {
  try { await navigator.clipboard.writeText(text); return true; } catch (_) { /* try the old way */ }
  try {
    const t = document.createElement('textarea');
    t.value = text;
    t.style.cssText = 'position:fixed;left:-9999px;top:0;';
    document.body.append(t);
    t.select();
    const ok = document.execCommand('copy');
    t.remove();
    return ok;
  } catch (_) { return false; }
}

/** A picture (File, Blob or data URL) as a JPEG data URL no bigger than `max` px on a side. */
export async function toJpegDataUrl(src, max = 1600) {
  const blob = typeof src === 'string' ? await (await fetch(src)).blob() : src;
  const bmp = await createImageBitmap(blob);
  const k = Math.min(1, max / Math.max(bmp.width, bmp.height));
  const c = document.createElement('canvas');
  c.width = Math.max(1, Math.round(bmp.width * k));
  c.height = Math.max(1, Math.round(bmp.height * k));
  const ctx = c.getContext('2d');
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(0, 0, c.width, c.height);
  ctx.drawImage(bmp, 0, 0, c.width, c.height);
  return c.toDataURL('image/jpeg', 0.85);
}

let mounted = null;
const draft = { images: [], usePage: false };
const MAX_PICTURES = 4;

function greeting(name) {
  const h = new Date().getHours();
  const g = h < 12 ? 'Good morning' : h < 18 ? 'Good afternoon' : 'Good evening';
  return name ? `${g}, ${name}` : g;
}

const SUGGESTIONS = [
  ['search', 'Search the web for today’s top tech news'],
  ['page', 'Summarise the page I have open'],
  ['pencil', 'Explain how a web browser shows a page, in simple steps'],
  ['tools', 'Find a good pasta recipe and open it for me'],
];

function lightbox(src) {
  const o = el('div', 'chat-lightbox');
  o.style.cssText = 'position:fixed;inset:0;z-index:400;background:rgba(0,0,0,.82);display:flex;align-items:center;justify-content:center;cursor:zoom-out;';
  const img = el('img');
  img.src = src;
  img.style.cssText = 'max-width:92vw;max-height:90vh;border-radius:10px;';
  o.append(img);
  const close = () => { o.remove(); window.removeEventListener('keydown', onKey, true); };
  const onKey = (e) => { if (e.key === 'Escape') { e.stopPropagation(); close(); } };
  o.addEventListener('click', close);
  window.addEventListener('keydown', onKey, true);
  document.body.append(o);
}

function thumbs(images, onRemove) {
  const row = el('div', 'chat-thumbs');
  row.style.cssText = 'display:flex;flex-wrap:wrap;gap:8px;';
  images.forEach((src, i) => {
    const w = el('div', 'chat-thumb');
    w.style.cssText = 'position:relative;width:72px;height:72px;border-radius:10px;overflow:hidden;background:var(--w-field);cursor:zoom-in;flex:none;';
    const img = el('img');
    img.src = src;
    img.alt = `Picture ${i + 1}`;
    img.style.cssText = 'width:100%;height:100%;object-fit:cover;display:block;';
    w.append(img);
    w.addEventListener('click', () => lightbox(src));
    if (onRemove) {
      const x = el('button', 'chat-thumb-x');
      x.type = 'button';
      x.setAttribute('aria-label', 'Remove picture');
      x.style.cssText = 'position:absolute;top:3px;right:3px;width:20px;height:20px;border-radius:10px;border:0;background:rgba(0,0,0,.65);color:#fff;display:flex;align-items:center;justify-content:center;cursor:pointer;padding:0;';
      x.append(icon('x', 11));
      x.addEventListener('click', (e) => { e.stopPropagation(); onRemove(i); });
      w.append(x);
    }
    row.append(w);
  });
  return row;
}

function addCopyButtons(body) {
  body.querySelectorAll('pre').forEach((pre) => {
    pre.style.position = 'relative';
    const bar = el('div', 'md-pre-bar');
    bar.style.cssText = 'position:absolute;top:6px;right:8px;display:flex;gap:8px;align-items:center;';
    if (pre.dataset.lang) { const l = el('span', null, pre.dataset.lang); l.style.cssText = 'font-size:11px;color:var(--text-muted);'; bar.append(l); }
    const b = el('button', 'md-copy', 'Copy');
    b.type = 'button';
    b.style.cssText = 'border:0;background:var(--w-field);color:var(--text-soft);font:inherit;font-size:11.5px;padding:2px 9px;border-radius:6px;cursor:pointer;';
    b.addEventListener('click', async () => {
      const ok = await copyText(pre.querySelector('code').textContent);
      b.textContent = ok ? 'Copied' : 'Failed';
      setTimeout(() => { b.textContent = 'Copy'; }, 1400);
    });
    bar.append(b);
    pre.append(bar);
  });
}

function stepsBlock(steps, done, hasText) {
  const d = el('details', 'ans-steps-box');
  d.style.cssText = 'margin-bottom:10px;';
  // open while it is working, collapsed once the answer is there
  d.open = !done && !hasText;
  const sum = el('summary');
  sum.style.cssText = 'cursor:pointer;color:var(--text-muted);font-size:12.5px;display:flex;align-items:center;gap:6px;list-style:none;';
  sum.append(icon('steps', 13), el('span', null, `${steps.length} ${steps.length === 1 ? 'step' : 'steps'}`));
  d.append(sum);
  const box = el('div', 'ans-steps');
  box.style.cssText = 'display:flex;flex-direction:column;gap:6px;margin:8px 0 0 4px;';
  steps.forEach((t, i) => {
    const active = !done && i === steps.length - 1 && !hasText;
    const r = el('div', `ans-step${active ? ' active' : ''}`);
    r.style.cssText = 'display:flex;align-items:center;gap:8px;font-size:13px;color:var(--text-soft);';
    const dot = el('span', 'ans-step-dot');
    dot.style.cssText = 'width:6px;height:6px;border-radius:3px;background:var(--accent);flex:none;' + (active ? '' : 'opacity:.45;');
    r.append(dot, el('span', null, t));
    box.append(r);
  });
  d.append(box);
  return d;
}

function fillAssistant(col, msg, ctx, done) {
  col.replaceChildren();
  const steps = msg.steps || [];
  if (steps.length) col.append(stepsBlock(steps, done, !!msg.text));
  if (!msg.text && !done) col.append(thinkingStars());
  if (msg.text) {
    const body = el('div', `md chat-answer${done ? '' : ' streaming'}`);
    body.style.cssText = 'font-size:15px;line-height:1.65;color:var(--text);';
    renderMarkdown(body, msg.text, ctx.actions.openLink);
    if (done) addCopyButtons(body);
    col.append(body);
  }
  if (msg.sources && msg.sources.length) {
    const row = el('div', 'ans-sources');
    row.style.cssText = 'display:flex;flex-wrap:wrap;gap:8px;margin-top:12px;';
    msg.sources.slice(0, 8).forEach((s) => {
      const chip = el(s.url ? 'button' : 'span', null, s.title || s.url);
      chip.style.cssText = 'height:26px;max-width:240px;padding:0 10px;border-radius:var(--r-chip);background:var(--w-field);display:inline-flex;align-items:center;font-size:12.5px;color:var(--text-soft);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;' + (s.url ? 'border:0;font:inherit;font-size:12.5px;cursor:pointer;' : '');
      if (s.url) chip.addEventListener('click', () => ctx.actions.openLink(s.url));
      row.append(chip);
    });
    col.append(row);
  }
}

function userEdit(row, index, msg, ctx) {
  row.replaceChildren();
  row.style.cssText = 'display:flex;justify-content:flex-end;';
  const box = el('div', 'chat-edit');
  box.style.cssText = 'width:min(80%,560px);display:flex;flex-direction:column;gap:8px;';
  if ((msg.images || []).length) box.append(thumbs(msg.images));
  const ta = el('textarea', 'chat-edit-input');
  ta.value = msg.text;
  ta.rows = 3;
  ta.style.cssText = 'width:100%;box-sizing:border-box;border:1px solid var(--accent);outline:0;resize:vertical;background:var(--w-field);color:var(--text);font:inherit;font-size:15px;line-height:1.5;padding:10px 12px;border-radius:var(--r-answer);';
  const btns = el('div');
  btns.style.cssText = 'display:flex;justify-content:flex-end;gap:8px;';
  const cancel = el('button', null, 'Cancel');
  cancel.style.cssText = 'border:0;background:none;color:var(--text-soft);font:inherit;font-size:13.5px;padding:7px 14px;border-radius:var(--r-row);cursor:pointer;';
  const save = el('button', 'chat-edit-save', 'Save and send');
  save.style.cssText = 'border:0;background:var(--accent);color:var(--on-accent);font:inherit;font-size:13.5px;font-weight:500;padding:7px 14px;border-radius:var(--r-row);cursor:pointer;';
  cancel.addEventListener('click', () => { const fresh = messageNode(msg, index, ctx, false); row.replaceWith(fresh); });
  save.addEventListener('click', () => {
    const text = ta.value.trim();
    if (!text) return;
    ctx.actions.send({ text, images: msg.images || [], editFrom: index });
  });
  ta.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); save.click(); }
    if (e.key === 'Escape') { e.stopPropagation(); cancel.click(); }
  });
  btns.append(cancel, save);
  box.append(ta, btns);
  row.append(box);
  setTimeout(() => { ta.focus(); ta.setSelectionRange(ta.value.length, ta.value.length); }, 0);
}

function messageNode(msg, index, ctx, animate) {
  const row = el('div', `chat-msg ${msg.role}${animate ? ' msg-in' : ''}`);
  row.dataset.index = String(index);
  if (msg.role === 'user') {
    row.style.cssText = 'display:flex;flex-direction:column;align-items:flex-end;gap:6px;';
    if ((msg.images || []).length) {
      const t = thumbs(msg.images);
      t.style.justifyContent = 'flex-end';
      row.append(t);
    }
    if (msg.text) {
      const bubble = el('div', 'chat-bubble', msg.text);
      bubble.style.cssText = 'max-width:80%;padding:10px 14px;border-radius:var(--r-answer);background:var(--w-field);font-size:15px;line-height:1.5;white-space:pre-wrap;word-break:break-word;';
      row.append(bubble);
    }
    const acts = el('div', 'chat-actions');
    acts.style.cssText = 'display:flex;gap:2px;opacity:0;transition:opacity var(--dur-fast) var(--ease-out);';
    acts.append(
      iconButton('copy', 'Copy', async (e) => { const ok = await copyText(msg.text); ctx.toast(ok ? 'Copied' : 'Could not copy'); e.currentTarget.blur(); }),
      iconButton('pencil', 'Edit and send again', () => userEdit(row, index, msg, ctx), 'chat-edit-btn'),
    );
    row.append(acts);
    row.addEventListener('mouseenter', () => { acts.style.opacity = '1'; });
    row.addEventListener('mouseleave', () => { acts.style.opacity = '0'; });
    return row;
  }
  row.style.cssText = 'display:flex;gap:12px;align-items:flex-start;';
  const av = el('div', 'chat-avatar');
  av.style.cssText = 'width:26px;height:26px;border-radius:13px;background:var(--w-field);display:flex;align-items:center;justify-content:center;color:var(--accent);flex:none;margin-top:2px;';
  av.append(icon(STAR, 13, true));
  const col = el('div');
  col.style.cssText = 'flex:1;min-width:0;';
  const content = el('div', 'chat-content');
  fillAssistant(content, msg, ctx, true);
  col.append(content);
  const acts = el('div', 'chat-actions');
  acts.style.cssText = 'display:flex;gap:2px;align-items:center;margin-top:6px;opacity:0;transition:opacity var(--dur-fast) var(--ease-out);';
  acts.append(
    iconButton('copy', 'Copy answer', async () => { const ok = await copyText(msg.text); ctx.toast(ok ? 'Copied' : 'Could not copy'); }),
    iconButton('refresh', 'Ask again', () => ctx.actions.send({ regenerate: true, text: ctx.lastUserText(index) }), 'chat-regen'),
  );
  if (msg.time) { const t = el('span', null, clock(msg.time)); t.style.cssText = 'font-size:11.5px;color:var(--text-muted);margin-left:6px;'; acts.append(t); }
  col.append(acts);
  row.addEventListener('mouseenter', () => { acts.style.opacity = '1'; });
  row.addEventListener('mouseleave', () => { acts.style.opacity = '0'; });
  row.append(av, col);
  return row;
}

/** Builds the page once. `actions` is provided by shell.js. */
export function renderChatPage(container, state, actions) {
  container.replaceChildren();
  let sideOpen = true;
  try { sideOpen = localStorage.getItem('chat_side') !== 'closed'; } catch (_) { /* default */ }

  const root = el('div', 'chat-page');
  root.style.cssText = 'position:absolute;inset:0;display:flex;overflow:hidden;color:var(--text);';

  // ---------------- history column
  const side = el('aside', 'chat-side');
  const applySide = () => { side.style.width = sideOpen ? '272px' : '0px'; side.style.padding = sideOpen ? '20px 12px 14px 16px' : '0'; side.style.opacity = sideOpen ? '1' : '0'; };
  side.style.cssText = 'flex:none;display:flex;flex-direction:column;box-sizing:border-box;overflow:hidden;transition:width var(--dur-slow) var(--ease-in-out),opacity var(--dur-base) var(--ease-out);';
  applySide();
  const top = el('div');
  top.style.cssText = 'display:flex;gap:8px;margin:36px 0 12px;';
  const newBtn = el('button', 'chat-new');
  newBtn.type = 'button';
  newBtn.style.cssText = 'flex:1;height:38px;border:0;border-radius:var(--r-row);background:var(--accent);color:var(--on-accent);font:inherit;font-size:14px;font-weight:500;cursor:pointer;display:flex;align-items:center;justify-content:center;gap:6px;white-space:nowrap;';
  newBtn.append(icon('plus', 15), el('span', null, 'New chat'));
  newBtn.addEventListener('click', () => actions.newChat());
  top.append(newBtn);
  const searchBox = el('div');
  searchBox.style.cssText = 'display:flex;align-items:center;gap:8px;height:34px;padding:0 10px;border-radius:var(--r-row);background:var(--w-field);color:var(--text-muted);margin-bottom:10px;';
  const search = el('input', 'chat-search');
  search.type = 'text';
  search.placeholder = 'Search chats';
  search.setAttribute('aria-label', 'Search chats');
  search.style.cssText = 'flex:1;min-width:0;border:0;outline:0;background:transparent;color:var(--text);font:inherit;font-size:13.5px;';
  searchBox.append(icon('search', 14), search);
  let searchTimer = 0;
  search.addEventListener('input', () => {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => actions.search(search.value.trim()), 180);
  });
  const list = el('div', 'chat-list');
  list.style.cssText = 'flex:1;overflow-y:auto;scrollbar-width:thin;display:flex;flex-direction:column;gap:1px;min-height:0;';
  const clearBtn = el('button', 'chat-clear', 'Clear all history');
  clearBtn.type = 'button';
  clearBtn.style.cssText = 'border:0;background:none;color:var(--text-muted);font:inherit;font-size:12.5px;cursor:pointer;text-align:left;padding:10px 8px 0;';
  clearBtn.addEventListener('click', () => {
    if (clearBtn.dataset.armed === '1') { actions.clearAll(); clearBtn.dataset.armed = ''; clearBtn.textContent = 'Clear all history'; }
    else { clearBtn.dataset.armed = '1'; clearBtn.textContent = 'Click again to delete all'; setTimeout(() => { clearBtn.dataset.armed = ''; clearBtn.textContent = 'Clear all history'; }, 3000); }
  });
  side.append(top, searchBox, list, clearBtn);

  // ---------------- conversation
  const main = el('div', 'chat-main');
  main.style.cssText = 'flex:1;min-width:0;display:flex;flex-direction:column;position:relative;';

  const head = el('div', 'chat-head');
  head.style.cssText = 'display:flex;align-items:center;gap:8px;padding:30px 24px 6px 16px;min-height:44px;box-sizing:border-box;';
  const toggle = iconButton('sidebar', 'Show or hide the chat list', () => { sideOpen = !sideOpen; try { localStorage.setItem('chat_side', sideOpen ? 'open' : 'closed'); } catch (_) { /* ignore */ } applySide(); }, 'chat-side-toggle');
  const title = el('div', 'chat-title');
  title.style.cssText = 'flex:1;min-width:0;font-size:15px;font-weight:500;color:var(--text-strong);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;padding:4px 8px;border-radius:var(--r-row);cursor:text;';
  title.title = 'Click to rename';
  const modelBtn = el('button', 'chat-model');
  modelBtn.type = 'button';
  modelBtn.style.cssText = 'border:0;background:var(--w-field);color:var(--text-soft);font:inherit;font-size:12.5px;padding:5px 10px;border-radius:var(--r-chip);cursor:pointer;display:flex;align-items:center;gap:6px;max-width:220px;';
  const toolsBtn = iconButton('tools', 'What the assistant may do', () => togglePopover('tools'), 'chat-tools-btn');
  const pinBtn = iconButton('pin', 'Pin this chat', () => { if (state.chat?.id) actions.pin(state.chat.id, !state.chat.pinned); }, 'chat-pin-btn');
  const moreBtn = iconButton('more', 'More', () => togglePopover('more'), 'chat-more-btn');
  head.append(toggle, title, modelBtn, toolsBtn, pinBtn, moreBtn);

  const pop = el('div', 'chat-pop');
  pop.style.cssText = 'display:none;position:absolute;z-index:60;min-width:220px;border-radius:var(--r-popover);background:var(--surface);box-shadow:var(--shadow-popover);padding:6px;';
  main.append(head, pop);
  const closePop = () => { pop.style.display = 'none'; pop.dataset.kind = ''; };
  const place = (anchor) => { const a = anchor.getBoundingClientRect(); const m = main.getBoundingClientRect(); pop.style.left = `${Math.max(8, Math.min(m.width - 240, a.right - m.left - 220))}px`; pop.style.top = `${a.bottom - m.top + 6}px`; };
  const item = (label, onClick, opts = {}) => {
    const b = el('button', 'chat-pop-item');
    b.type = 'button';
    b.style.cssText = 'width:100%;border:0;background:none;color:var(--text);font:inherit;font-size:13.5px;text-align:left;padding:8px 10px;border-radius:var(--r-row);cursor:pointer;display:flex;align-items:center;gap:8px;';
    if (opts.icon) b.append(icon(opts.icon, 14));
    const t = el('span', null, label); t.style.cssText = 'flex:1;min-width:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;';
    b.append(t);
    if (opts.check) b.append(icon('check', 14));
    if (opts.danger) b.style.color = 'var(--text-soft)';
    b.addEventListener('mouseenter', () => { b.style.background = 'var(--w-hover-row)'; });
    b.addEventListener('mouseleave', () => { b.style.background = 'none'; });
    b.addEventListener('click', () => { closePop(); onClick(); });
    return b;
  };
  const switchRow = (label, hint, on, onChange) => {
    const r = el('div', 'chat-pop-switch');
    r.style.cssText = 'display:flex;align-items:center;gap:12px;padding:8px 10px;';
    const t = el('div'); t.style.cssText = 'flex:1;';
    const l = el('div', null, label); l.style.cssText = 'font-size:13.5px;';
    const h = el('div', null, hint); h.style.cssText = 'font-size:12px;color:var(--text-muted);margin-top:2px;line-height:1.4;';
    t.append(l, h);
    const sw = el('button');
    sw.type = 'button';
    sw.setAttribute('role', 'switch');
    sw.setAttribute('aria-checked', on ? 'true' : 'false');
    sw.setAttribute('aria-label', label);
    sw.style.cssText = `border:0;padding:0;cursor:pointer;width:38px;height:22px;border-radius:11px;position:relative;flex:none;background:${on ? 'var(--accent)' : 'var(--w-toggle-off)'};`;
    const k = el('span');
    k.style.cssText = `position:absolute;top:2px;width:18px;height:18px;border-radius:9px;background:var(--white);left:${on ? '18px' : '2px'};transition:left .15s ease;`;
    sw.append(k);
    sw.addEventListener('click', () => onChange(!on));
    r.append(t, sw);
    return r;
  };
  function togglePopover(kind) {
    if (pop.style.display === 'block' && pop.dataset.kind === kind) { closePop(); return; }
    pop.replaceChildren();
    pop.dataset.kind = kind;
    const set = state.settings || {};
    if (kind === 'model') {
      const ms = state.aiModels?.models || [];
      if (state.aiModels?.error) { const e = el('div', null, state.aiModels.error); e.style.cssText = 'padding:8px 10px;font-size:12.5px;color:var(--text-muted);max-width:260px;'; pop.append(e); }
      if (!ms.length && !state.aiModels?.error) { const e = el('div', null, 'Loading models…'); e.style.cssText = 'padding:8px 10px;font-size:12.5px;color:var(--text-muted);'; pop.append(e); }
      ms.forEach((name) => pop.append(item(name, () => actions.setModel(name), { check: name === state.aiConfig?.model })));
      place(modelBtn);
    } else if (kind === 'tools') {
      pop.append(
        switchRow('Search and open pages', 'Search the web, read pages, open tabs.', set.aiTools !== false, (v) => { actions.setSetting('aiTools', v); closePop(); }),
        switchRow('Control the page', 'Click, type and scroll on your last web tab. It asks before risky clicks.', set.aiControl !== false && set.aiTools !== false, (v) => { actions.setSetting('aiControl', v); closePop(); }),
      );
      place(toolsBtn);
    } else {
      pop.append(
        item('Save as Markdown', () => actions.exportChat(state.chat.id), { icon: 'download' }),
        item('Delete this chat', () => actions.deleteChat(state.chat.id), { icon: 'trash', danger: true }),
      );
      place(moreBtn);
    }
    pop.style.display = 'block';
  }
  modelBtn.addEventListener('click', () => {
    if (state.aiConfig?.provider === 'local') { actions.listModels(); togglePopover('model'); }
  });
  document.addEventListener('pointerdown', (e) => { if (pop.style.display === 'block' && !pop.contains(e.target) && !head.contains(e.target)) closePop(); }, true);

  title.addEventListener('click', () => {
    if (!state.chat?.id) return;
    const input = el('input', 'chat-title-input');
    input.value = state.chat.title || '';
    input.maxLength = 80;
    input.style.cssText = 'flex:1;min-width:0;font:inherit;font-size:15px;font-weight:500;color:var(--text-strong);background:var(--w-field);border:1px solid var(--accent);outline:0;border-radius:var(--r-row);padding:3px 8px;';
    const done = (save) => { if (save && input.value.trim() && input.value.trim() !== state.chat.title) actions.rename(state.chat.id, input.value.trim()); input.replaceWith(title); };
    input.addEventListener('keydown', (e) => { if (e.key === 'Enter') done(true); if (e.key === 'Escape') { e.stopPropagation(); done(false); } });
    input.addEventListener('blur', () => done(true));
    title.replaceWith(input);
    input.focus(); input.select();
  });

  const scroll = el('div', 'chat-scroll');
  scroll.style.cssText = 'flex:1;overflow-y:auto;scrollbar-width:thin;padding:14px 24px 12px;position:relative;';
  const column = el('div', 'chat-column');
  column.style.cssText = 'max-width:720px;margin:0 auto;display:flex;flex-direction:column;gap:22px;min-height:100%;';
  scroll.append(column);
  main.append(scroll);

  const jump = el('button', 'chat-jump');
  jump.type = 'button';
  jump.setAttribute('aria-label', 'Scroll to the latest message');
  jump.style.cssText = 'position:absolute;left:50%;transform:translateX(-50%);bottom:176px;width:34px;height:34px;border-radius:17px;border:0;background:var(--surface);color:var(--text);box-shadow:var(--shadow-popover);cursor:pointer;display:none;align-items:center;justify-content:center;z-index:20;';
  jump.append(icon('down', 16));
  jump.addEventListener('click', () => { scroll.scrollTo({ top: scroll.scrollHeight, behavior: 'smooth' }); });
  scroll.addEventListener('scroll', () => { jump.style.display = scroll.scrollHeight - scroll.scrollTop - scroll.clientHeight > 320 ? 'flex' : 'none'; });
  main.append(jump);

  // ---------------- composer
  const bar = el('div', 'chat-inputbar');
  bar.style.cssText = 'padding:0 24px 104px;'; // clear of the floating Helm
  const form = el('div', 'chat-form');
  form.style.cssText = 'max-width:720px;margin:0 auto;display:flex;flex-direction:column;gap:8px;background:var(--surface);border-radius:var(--r-answer);padding:10px 10px 8px 16px;box-shadow:var(--shadow-popover);';
  const attach = el('div', 'chat-attach');
  attach.style.cssText = 'display:none;padding-top:2px;';
  const ta = el('textarea', 'chat-input');
  ta.rows = 1;
  ta.placeholder = 'Message your assistant';
  ta.setAttribute('aria-label', 'Message');
  ta.style.cssText = 'border:0;outline:0;resize:none;background:transparent;color:var(--text);font:inherit;font-size:15px;line-height:1.5;padding:4px 0;max-height:160px;caret-color:var(--accent);width:100%;';
  const tools = el('div');
  tools.style.cssText = 'display:flex;align-items:center;gap:4px;';
  const file = el('input');
  file.type = 'file'; file.accept = 'image/*'; file.multiple = true;
  file.style.display = 'none';
  file.setAttribute('aria-label', 'Attach pictures');
  const clipBtn = iconButton('clip', 'Attach pictures', () => file.click(), 'chat-attach-btn');
  const capBtn = iconButton('camera', 'Screenshot of the page you were on', () => toggleCapMenu(), 'chat-capture-btn');
  const pageChip = el('button', 'chat-page-chip');
  pageChip.type = 'button';
  pageChip.style.cssText = 'border:0;background:none;color:var(--text-muted);font:inherit;font-size:12.5px;padding:4px 10px;border-radius:var(--r-chip);cursor:pointer;display:flex;align-items:center;gap:6px;';
  pageChip.append(icon('page', 14), el('span', null, 'This page'));
  pageChip.title = 'Let the assistant read the web page you were on';
  pageChip.addEventListener('click', () => { draft.usePage = !draft.usePage; refreshComposer(); });
  const spacer = el('div'); spacer.style.flex = '1';
  const sendBtn = el('button', 'chat-send');
  sendBtn.type = 'button';
  sendBtn.setAttribute('aria-label', 'Send');
  sendBtn.style.cssText = 'width:36px;height:36px;border-radius:18px;border:0;background:var(--accent);color:var(--on-accent);display:flex;align-items:center;justify-content:center;cursor:pointer;flex:none;';
  tools.append(clipBtn, capBtn, pageChip, spacer, sendBtn);
  form.append(attach, ta, tools, file);
  bar.append(form);
  main.append(bar);

  const capMenu = el('div', 'chat-capmenu');
  capMenu.style.cssText = 'display:none;position:absolute;z-index:70;min-width:230px;border-radius:var(--r-popover);background:var(--surface);box-shadow:var(--shadow-popover);padding:6px;';
  main.append(capMenu);
  function toggleCapMenu() {
    if (capMenu.style.display === 'block') { capMenu.style.display = 'none'; return; }
    capMenu.replaceChildren();
    const pick = (label, full) => {
      const b = el('button', 'chat-cap-item', label);
      b.type = 'button';
      b.style.cssText = 'width:100%;border:0;background:none;color:var(--text);font:inherit;font-size:13.5px;text-align:left;padding:8px 10px;border-radius:var(--r-row);cursor:pointer;';
      b.addEventListener('mouseenter', () => { b.style.background = 'var(--w-hover-row)'; });
      b.addEventListener('mouseleave', () => { b.style.background = 'none'; });
      b.addEventListener('click', () => { capMenu.style.display = 'none'; actions.capture(full); });
      return b;
    };
    capMenu.append(pick('Visible part of your web page', false), pick('The whole page', true));
    const a = capBtn.getBoundingClientRect(); const m = main.getBoundingClientRect();
    capMenu.style.left = `${Math.max(8, a.left - m.left)}px`;
    capMenu.style.bottom = `${m.bottom - a.top + 8}px`;
    capMenu.style.display = 'block';
  }
  document.addEventListener('pointerdown', (e) => { if (capMenu.style.display === 'block' && !capMenu.contains(e.target) && !capBtn.contains(e.target)) capMenu.style.display = 'none'; }, true);

  const toastEl = el('div', 'chat-toast');
  toastEl.style.cssText = 'position:absolute;left:50%;transform:translateX(-50%);bottom:190px;background:var(--surface);color:var(--text);box-shadow:var(--shadow-popover);border-radius:var(--r-chip);padding:7px 14px;font-size:13px;display:none;z-index:80;max-width:80%;';
  main.append(toastEl);
  let toastTimer = 0;
  const toast = (text) => { toastEl.textContent = text; toastEl.style.display = 'block'; clearTimeout(toastTimer); toastTimer = setTimeout(() => { toastEl.style.display = 'none'; }, 2400); };

  // drop pictures anywhere on the page
  const dropVeil = el('div', 'chat-drop');
  dropVeil.style.cssText = 'display:none;position:absolute;inset:8px;border:2px dashed var(--accent);border-radius:var(--r-answer);background:var(--w-field);z-index:90;align-items:center;justify-content:center;font-size:16px;color:var(--text);pointer-events:none;';
  dropVeil.textContent = 'Drop pictures to attach';
  main.append(dropVeil);
  let dragDepth = 0;
  root.addEventListener('dragenter', (e) => { if (e.dataTransfer?.types?.includes('Files')) { dragDepth++; dropVeil.style.display = 'flex'; } });
  root.addEventListener('dragleave', () => { dragDepth = Math.max(0, dragDepth - 1); if (!dragDepth) dropVeil.style.display = 'none'; });
  root.addEventListener('dragover', (e) => { if (e.dataTransfer?.types?.includes('Files')) e.preventDefault(); });
  root.addEventListener('drop', (e) => {
    dragDepth = 0; dropVeil.style.display = 'none';
    if (!e.dataTransfer?.files?.length) return;
    e.preventDefault();
    addFiles([...e.dataTransfer.files]);
  });

  async function addFiles(files) {
    const imgs = files.filter((f) => f.type.startsWith('image/'));
    if (!imgs.length) { toast('Only pictures can be attached.'); return; }
    for (const f of imgs) {
      if (draft.images.length >= MAX_PICTURES) { toast(`At most ${MAX_PICTURES} pictures.`); break; }
      try { draft.images.push(await toJpegDataUrl(f)); } catch (_) { toast('That picture could not be read.'); }
    }
    refreshComposer();
  }
  file.addEventListener('change', () => { addFiles([...file.files]); file.value = ''; });
  ta.addEventListener('paste', (e) => {
    const files = [...(e.clipboardData?.files || [])].filter((f) => f.type.startsWith('image/'));
    if (files.length) { e.preventDefault(); addFiles(files); }
  });

  const grow = () => { ta.style.height = 'auto'; ta.style.height = `${Math.min(160, ta.scrollHeight)}px`; };
  const busyNow = () => !!(state.chatStream && !state.chatStream.done);
  const submit = () => {
    const text = ta.value.trim();
    if ((!text && !draft.images.length) || busyNow()) return;
    const payload = { text: text || 'What is in this picture?', images: draft.images.slice(), usePage: draft.usePage };
    ta.value = ''; grow();
    draft.images = []; draft.usePage = false;
    refreshComposer();
    actions.send(payload);
  };
  ta.addEventListener('input', grow);
  ta.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); submit(); }
    else if (e.key === 'Escape' && busyNow()) { e.stopPropagation(); actions.stop(); }
    else if (e.key === 'ArrowUp' && !ta.value) {
      const msgs = state.chat?.messages || [];
      for (let i = msgs.length - 1; i >= 0; i--) if (msgs[i].role === 'user') { const n = column.querySelector(`.chat-msg[data-index="${i}"] .chat-edit-btn`); if (n) { e.preventDefault(); n.click(); } break; }
    }
  });
  sendBtn.addEventListener('click', () => { if (busyNow()) actions.stop(); else submit(); });

  function refreshComposer() {
    attach.replaceChildren();
    attach.style.display = draft.images.length ? 'block' : 'none';
    if (draft.images.length) attach.append(thumbs(draft.images, (i) => { draft.images.splice(i, 1); refreshComposer(); }));
    pageChip.style.background = draft.usePage ? 'var(--w-field)' : 'none';
    pageChip.style.color = draft.usePage ? 'var(--accent)' : 'var(--text-muted)';
    pageChip.setAttribute('aria-pressed', draft.usePage ? 'true' : 'false');
    const busy = busyNow();
    sendBtn.replaceChildren(icon(busy ? 'stop' : 'up', busy ? 14 : 18, busy));
    sendBtn.setAttribute('aria-label', busy ? 'Stop' : 'Send');
    sendBtn.classList.toggle('busy', busy);
  }

  root.append(side, main);
  container.append(root);
  mounted = { root, list, column, scroll, ta, search, title, modelBtn, pinBtn, moreBtn, sideList: list, actions, rendered: 0, lastChat: null, streamNode: null, refreshComposer, toast, addFiles, closePop, head };
  mounted.ctx = {
    actions,
    toast,
    lastUserText: (i) => { const msgs = mountedState.chat?.messages || []; for (let k = Math.min(i, msgs.length) - 1; k >= 0; k--) if (msgs[k].role === 'user') return msgs[k].text; return ''; },
  };
  mountedState = state;
  chatUpdate(state);
  grow();
  actions.listModels?.();
  setTimeout(() => ta.focus(), 50);
  return root;
}

let mountedState = null;

function stick(scroll) {
  const near = scroll.scrollHeight - scroll.scrollTop - scroll.clientHeight < 140;
  return () => { if (near) scroll.scrollTop = scroll.scrollHeight; };
}

function renderList(m, state) {
  const { actions } = m;
  const query = state.chatQuery || '';
  const chats = query ? (state.chatFound || []) : (state.chatList || []);
  const key = JSON.stringify([chats.map((c) => [c.id, c.title, c.updated, c.pinned, c.snippet]), state.chat?.id, query]);
  if (m.listKey === key) return;
  m.listKey = key;
  m.list.replaceChildren();
  if (!chats.length) {
    const e = el('div', null, query ? 'No chats match.' : 'Your conversations will appear here.');
    e.style.cssText = 'font-size:13px;color:var(--text-muted);padding:8px;line-height:1.5;';
    m.list.append(e);
    return;
  }
  let lastGroup = '';
  for (const c of chats) {
    const g = query ? 'Results' : dayGroup(c.updated, c.pinned);
    if (g !== lastGroup) {
      lastGroup = g;
      const h = el('div', 'chat-group', g);
      h.style.cssText = 'font-size:11.5px;letter-spacing:.2px;color:var(--text-muted);margin:12px 8px 4px;';
      m.list.append(h);
    }
    const current = c.id === state.chat?.id;
    const row = el('div', `chat-item${current ? ' current' : ''}`);
    row.dataset.id = c.id;
    row.style.cssText = `display:flex;align-items:center;gap:2px;border-radius:var(--r-row);${current ? 'background:var(--w-active-row);' : ''}`;
    const open = el('button', 'chat-item-open');
    open.type = 'button';
    open.style.cssText = 'flex:1;min-width:0;border:0;background:none;color:inherit;font:inherit;text-align:left;cursor:pointer;padding:8px 10px;display:flex;flex-direction:column;gap:2px;';
    const t = el('span', null, (c.pinned ? '• ' : '') + c.title);
    t.style.cssText = 'font-size:13.5px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;';
    const sub = el('span', null, c.snippet || relativeTime(c.updated));
    sub.style.cssText = 'font-size:11.5px;color:var(--text-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;';
    open.append(t, sub);
    open.addEventListener('click', () => actions.openChat(c.id));
    const acts = el('div', 'chat-item-acts');
    acts.style.cssText = 'display:flex;opacity:0;flex:none;';
    acts.append(
      iconButton('pin', c.pinned ? 'Unpin' : 'Pin', () => actions.pin(c.id, !c.pinned)),
      iconButton('trash', `Delete ${c.title}`, () => actions.deleteChat(c.id), 'chat-item-delete'),
    );
    row.addEventListener('mouseenter', () => { acts.style.opacity = '1'; });
    row.addEventListener('mouseleave', () => { acts.style.opacity = '0'; });
    row.append(open, acts);
    m.list.append(row);
  }
}

/** Brings the mounted page up to date with `state` (no-op when the page is not shown). */
export function chatUpdate(state) {
  const m = mounted;
  if (!m || !m.root.isConnected) return;
  mountedState = state;
  const { actions } = m;
  const busy = !!(state.chatStream && !state.chatStream.done);

  renderList(m, state);

  // header
  if (m.title.isConnected) m.title.textContent = state.chat?.title || 'New chat';
  const cfg = state.aiConfig || {};
  m.modelBtn.replaceChildren();
  const mn = el('span', null, cfg.model ? cfg.model.replace(/^.*\//, '') : (cfg.provider && cfg.provider !== 'none' ? cfg.provider : 'No model'));
  mn.style.cssText = 'white-space:nowrap;overflow:hidden;text-overflow:ellipsis;';
  m.modelBtn.append(mn);
  if (cfg.provider === 'local') m.modelBtn.append(icon('chevron', 13));
  m.modelBtn.style.cursor = cfg.provider === 'local' ? 'pointer' : 'default';
  m.pinBtn.style.display = state.chat?.id ? '' : 'none';
  m.moreBtn.style.display = state.chat?.id ? '' : 'none';
  m.pinBtn.style.color = state.chat?.pinned ? 'var(--accent)' : '';
  m.pinBtn.setAttribute('aria-label', state.chat?.pinned ? 'Unpin this chat' : 'Pin this chat');

  // messages
  const messages = state.chat?.messages || [];
  const stayAtBottom = stick(m.scroll);
  const chatKey = state.chat?.id ?? 'new';
  if (m.lastChat !== chatKey) {
    // A new conversation that just got its id keeps what is on screen; any other switch redraws.
    if (m.adopt !== chatKey) {
      m.column.replaceChildren();
      m.rendered = 0;
      m.streamNode = null;
      draft.usePage = false;
    }
    m.adopt = null;
    m.lastChat = chatKey;
  }
  // an edit or "ask again" shortens the conversation: redraw from the top
  if (messages.length < m.rendered) { m.column.replaceChildren(); m.rendered = 0; m.streamNode = null; }
  if (m.streamNode && (messages.length > m.rendered || !state.chatStream)) {
    m.streamNode.remove();
    m.streamNode = null;
  }
  const empty = m.column.querySelector('.chat-empty');
  if ((messages.length || state.chatStream) && empty) empty.remove();
  // only the newest answer can be asked again
  m.column.querySelectorAll('.chat-regen').forEach((b) => { b.style.display = 'none'; });
  for (let i = m.rendered; i < messages.length; i++) {
    m.column.append(messageNode(messages[i], i, m.ctx, m.rendered > 0 || state.chatStream != null || m.animateNext));
  }
  m.rendered = messages.length;
  m.animateNext = false;
  const lastRow = m.column.querySelector('.chat-msg.assistant:last-of-type .chat-regen');
  const lastIsAssistant = messages.length && messages[messages.length - 1].role === 'assistant';
  if (lastRow && lastIsAssistant && !busy) lastRow.style.display = '';

  if (!messages.length && !state.chatStream && !m.column.querySelector('.chat-empty')) {
    const e = el('div', 'chat-empty');
    e.style.cssText = 'margin:auto;display:flex;flex-direction:column;align-items:center;gap:16px;text-align:center;padding:40px 0;';
    const stars = thinkingStars();
    stars.querySelector('.think-label').remove();
    stars.classList.add('calm');
    const h = el('div', null, greeting(state.settings?.greetingName));
    h.style.cssText = 'font-size:30px;font-weight:600;letter-spacing:-.5px;color:var(--text-strong);';
    const sub = el('div', null, 'How can I help? I can search the web, read pages, look at pictures and work in your tabs.');
    sub.style.cssText = 'font-size:14.5px;color:var(--text-muted);max-width:460px;line-height:1.5;';
    const grid = el('div');
    grid.style.cssText = 'display:grid;grid-template-columns:1fr 1fr;gap:10px;margin-top:10px;width:min(560px,100%);';
    for (const [ic, s] of SUGGESTIONS) {
      const b = el('button', 'chat-suggestion');
      b.type = 'button';
      b.style.cssText = 'border:0;background:var(--w-field);color:var(--text-soft);font:inherit;font-size:13.5px;text-align:left;padding:12px 14px;border-radius:var(--r-answer);cursor:pointer;display:flex;gap:10px;align-items:flex-start;line-height:1.4;';
      const ico = icon(ic, 16);
      ico.style.cssText = 'flex:none;margin-top:1px;color:var(--accent);';
      b.append(ico, el('span', null, s));
      b.addEventListener('click', () => { if (ic === 'page') draft.usePage = true; actions.send({ text: s, images: [], usePage: ic === 'page' }); draft.usePage = false; m.refreshComposer(); });
      grid.append(b);
    }
    e.append(stars, h, sub, grid);
    m.column.append(e);
  }

  // the answer being written
  const stream = state.chatStream;
  if (stream && !stream.done) {
    if (!m.streamNode) {
      m.streamNode = messageNode({ role: 'assistant', text: '' }, messages.length, m.ctx, true);
      m.streamNode.querySelector('.chat-actions')?.remove();
      m.column.append(m.streamNode);
    }
    fillAssistant(m.streamNode.querySelector('.chat-content'), stream, m.ctx, false);
  } else if (stream && stream.done && stream.error && !m.streamNode) {
    m.streamNode = messageNode({ role: 'assistant', text: stream.error }, messages.length, m.ctx, true);
    m.column.append(m.streamNode);
  }

  m.refreshComposer();
  stayAtBottom();
}

/** A picture of the page arrived (the camera button). */
export async function chatCaptured(msg) {
  const m = mounted;
  if (!m) return;
  if (!msg.ok) { m.toast(msg.message || 'The page could not be captured.'); return; }
  if (draft.images.length >= MAX_PICTURES) { m.toast(`At most ${MAX_PICTURES} pictures.`); return; }
  try {
    // for automated checks: the size of the picture as it arrived
    const probe = new Image();
    probe.src = msg.data;
    await probe.decode();
    window.__lastCapture = [probe.naturalWidth, probe.naturalHeight];
    draft.images.push(await toJpegDataUrl(msg.data, 1800));
    m.toast(msg.message || (msg.title ? `Added a screenshot of “${msg.title.slice(0, 40)}”` : 'Added a screenshot'));
  } catch (_) { m.toast('The screenshot could not be used.'); }
  m.refreshComposer();
}

export function chatToast(text) { mounted?.toast(text); }
/** Called when a different page replaces the chat page. */
export function chatUnmount() { mounted = null; }
export function chatFocusInput() { mounted?.ta.focus(); }
export function chatAdopt(id) { if (mounted) mounted.adopt = id; }
export function chatAnimateNext() { if (mounted) mounted.animateNext = true; }
export function chatDraft() { return draft; }
