// The assistant page (odyssey://chat): a dedicated chat with your assistant, with every earlier
// conversation listed on the left. The DOM is built once and updated in place (chatUpdate), so
// streaming an answer never loses what you are typing or where you scrolled.

import { renderMarkdown } from './markdown.js';
import { thinkingStars } from './helm.js';

const SVG = 'http://www.w3.org/2000/svg';
const STAR = 'M12 2C12.8 8.5 15.5 11.2 22 12C15.5 12.8 12.8 15.5 12 22C11.2 15.5 8.5 12.8 2 12C8.5 11.2 11.2 8.5 12 2Z';

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

function icon(d, size = 16, fill = false) {
  const s = document.createElementNS(SVG, 'svg');
  for (const [k, v] of Object.entries({ width: size, height: size, viewBox: '0 0 24 24', 'aria-hidden': 'true', fill: fill ? 'currentColor' : 'none', stroke: fill ? 'none' : 'currentColor', 'stroke-width': '1.8', 'stroke-linecap': 'round', 'stroke-linejoin': 'round' })) s.setAttribute(k, v);
  const p = document.createElementNS(SVG, 'path');
  p.setAttribute('d', d);
  s.append(p);
  return s;
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

let mounted = null; // { root, refs, actions }

const SUGGESTIONS = [
  'Search the web for today’s top tech news',
  'Explain how a web browser shows a page, in simple steps',
  'Find a good pasta recipe and open it for me',
];

function messageNode(m, onLink, animate) {
  const row = el('div', `chat-msg ${m.role}${animate ? ' msg-in' : ''}`);
  if (m.role === 'user') {
    row.style.cssText = 'display:flex;justify-content:flex-end;';
    const bubble = el('div', 'chat-bubble', m.text);
    bubble.style.cssText = 'max-width:80%;padding:10px 14px;border-radius:var(--r-answer);background:var(--w-field);font-size:15px;line-height:1.5;white-space:pre-wrap;word-break:break-word;';
    row.append(bubble);
    return row;
  }
  row.style.cssText = 'display:flex;gap:12px;align-items:flex-start;';
  const av = el('div', 'chat-avatar');
  av.style.cssText = 'width:26px;height:26px;border-radius:13px;background:var(--w-field);display:flex;align-items:center;justify-content:center;color:var(--accent);flex:none;margin-top:2px;';
  av.append(icon(STAR, 13, true));
  const col = el('div');
  col.style.cssText = 'flex:1;min-width:0;';
  fillAssistant(col, m, onLink, true);
  row.append(av, col);
  return row;
}

function fillAssistant(col, m, onLink, done) {
  col.replaceChildren();
  const steps = m.steps || [];
  if (steps.length) {
    const box = el('div', 'ans-steps');
    box.style.cssText = 'display:flex;flex-direction:column;gap:6px;margin-bottom:10px;';
    steps.forEach((t, i) => {
      const active = !done && i === steps.length - 1 && !m.text;
      const r = el('div', `ans-step${active ? ' active' : ''}`);
      r.style.cssText = 'display:flex;align-items:center;gap:8px;font-size:13px;color:var(--text-soft);';
      const dot = el('span', 'ans-step-dot');
      dot.style.cssText = 'width:6px;height:6px;border-radius:3px;background:var(--accent);flex:none;' + (active ? '' : 'opacity:.45;');
      r.append(dot, el('span', null, t));
      box.append(r);
    });
    col.append(box);
  }
  if (!m.text && !done) col.append(thinkingStars());
  if (m.text) {
    const body = el('div', 'md');
    body.style.cssText = 'font-size:15px;line-height:1.65;color:var(--text);';
    renderMarkdown(body, m.text, onLink);
    col.append(body);
  }
  if (m.sources && m.sources.length) {
    const row = el('div', 'ans-sources');
    row.style.cssText = 'display:flex;flex-wrap:wrap;gap:8px;margin-top:12px;';
    m.sources.slice(0, 8).forEach((s) => {
      const chip = el(s.url ? 'button' : 'span', null, s.title || s.url);
      chip.style.cssText = 'height:26px;max-width:240px;padding:0 10px;border-radius:var(--r-chip);background:var(--w-field);display:inline-flex;align-items:center;font-size:12.5px;color:var(--text-soft);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;' + (s.url ? 'border:0;font:inherit;font-size:12.5px;cursor:pointer;' : '');
      if (s.url) chip.addEventListener('click', () => onLink(s.url));
      row.append(chip);
    });
    col.append(row);
  }
}

/** Builds the page once. `actions`: send(prompt), stop(), openChat(id), newChat(), deleteChat(id), clearAll(), openLink(url). */
export function renderChatPage(container, state, actions) {
  container.replaceChildren();
  const root = el('div', 'chat-page');
  root.style.cssText = 'position:absolute;inset:0;display:flex;overflow:hidden;color:var(--text);';

  // --- history column
  const side = el('aside', 'chat-side');
  side.style.cssText = 'width:260px;flex:none;display:flex;flex-direction:column;padding:56px 12px 16px 16px;box-sizing:border-box;';
  const newBtn = el('button', 'chat-new', 'New chat');
  newBtn.style.cssText = 'height:38px;border:0;border-radius:var(--r-row);background:var(--accent);color:var(--on-accent);font:inherit;font-size:14px;font-weight:500;cursor:pointer;margin-bottom:14px;';
  newBtn.addEventListener('click', () => actions.newChat());
  const listTitle = el('div', null, 'History');
  listTitle.style.cssText = 'font-size:12.5px;color:var(--text-muted);margin:4px 8px 6px;';
  const list = el('div', 'chat-list');
  list.style.cssText = 'flex:1;overflow-y:auto;scrollbar-width:thin;display:flex;flex-direction:column;gap:2px;';
  const clearBtn = el('button', 'chat-clear', 'Clear all history');
  clearBtn.style.cssText = 'border:0;background:none;color:var(--text-muted);font:inherit;font-size:12.5px;cursor:pointer;text-align:left;padding:10px 8px 0;';
  clearBtn.addEventListener('click', () => {
    if (clearBtn.dataset.armed === '1') { actions.clearAll(); clearBtn.dataset.armed = ''; clearBtn.textContent = 'Clear all history'; }
    else { clearBtn.dataset.armed = '1'; clearBtn.textContent = 'Click again to delete all'; setTimeout(() => { clearBtn.dataset.armed = ''; clearBtn.textContent = 'Clear all history'; }, 3000); }
  });
  side.append(newBtn, listTitle, list, clearBtn);

  // --- conversation
  const main = el('div', 'chat-main');
  main.style.cssText = 'flex:1;min-width:0;display:flex;flex-direction:column;position:relative;';
  const scroll = el('div', 'chat-scroll');
  scroll.style.cssText = 'flex:1;overflow-y:auto;scrollbar-width:thin;padding:56px 24px 12px;';
  const column = el('div', 'chat-column');
  column.style.cssText = 'max-width:720px;margin:0 auto;display:flex;flex-direction:column;gap:22px;min-height:100%;';
  scroll.append(column);

  const bar = el('div', 'chat-inputbar');
  bar.style.cssText = 'padding:0 24px 104px;'; // clear of the floating Helm
  const form = el('div', 'chat-form');
  form.style.cssText = 'max-width:720px;margin:0 auto;display:flex;align-items:flex-end;gap:10px;background:var(--surface);border-radius:var(--r-helm);padding:8px 8px 8px 20px;box-shadow:var(--shadow-popover);';
  const ta = el('textarea', 'chat-input');
  ta.rows = 1;
  ta.placeholder = 'Message your assistant';
  ta.setAttribute('aria-label', 'Message');
  ta.style.cssText = 'flex:1;border:0;outline:0;resize:none;background:transparent;color:var(--text);font:inherit;font-size:15px;line-height:1.5;padding:7px 0;max-height:160px;caret-color:var(--accent);';
  const sendBtn = el('button', 'chat-send');
  sendBtn.setAttribute('aria-label', 'Send');
  sendBtn.style.cssText = 'width:38px;height:38px;border-radius:19px;border:0;background:var(--accent);color:var(--on-accent);display:flex;align-items:center;justify-content:center;cursor:pointer;flex:none;';
  form.append(ta, sendBtn);
  bar.append(form);
  main.append(scroll, bar);
  root.append(side, main);
  container.append(root);

  const grow = () => { ta.style.height = 'auto'; ta.style.height = `${Math.min(160, ta.scrollHeight)}px`; };
  const submit = () => {
    const text = ta.value.trim();
    if (!text) return;
    if (state.chatStream && !state.chatStream.done) return;
    ta.value = '';
    grow();
    actions.send(text);
  };
  ta.addEventListener('input', grow);
  ta.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) { e.preventDefault(); submit(); }
  });
  sendBtn.addEventListener('click', () => {
    if (state.chatStream && !state.chatStream.done) actions.stop(); else submit();
  });

  mounted = { root, list, column, scroll, ta, sendBtn, clearBtn, actions, rendered: 0, lastChat: null, streamNode: null };
  chatUpdate(state);
  grow();
  setTimeout(() => ta.focus(), 50);
  return root;
}

function stick(scroll) {
  const near = scroll.scrollHeight - scroll.scrollTop - scroll.clientHeight < 140;
  return () => { if (near) scroll.scrollTop = scroll.scrollHeight; };
}

/** Brings the mounted page up to date with `state` (no-op when the page is not shown). */
export function chatUpdate(state) {
  const m = mounted;
  if (!m || !m.root.isConnected) return;
  const { actions } = m;
  const busy = !!(state.chatStream && !state.chatStream.done);

  // history list
  const chats = state.chatList || [];
  const key = JSON.stringify([chats.map((c) => [c.id, c.title, c.updated]), state.chat?.id]);
  if (m.listKey !== key) {
    m.listKey = key;
    m.list.replaceChildren();
    if (!chats.length) {
      const e = el('div', null, 'Your conversations will appear here.');
      e.style.cssText = 'font-size:13px;color:var(--text-muted);padding:8px;line-height:1.5;';
      m.list.append(e);
    }
    for (const c of chats) {
      const row = el('div', `chat-item${c.id === state.chat?.id ? ' current' : ''}`);
      row.dataset.id = c.id;
      row.style.cssText = `display:flex;align-items:center;gap:4px;border-radius:var(--r-row);${c.id === state.chat?.id ? 'background:var(--w-active-row);' : ''}`;
      const open = el('button', 'chat-item-open');
      open.style.cssText = 'flex:1;min-width:0;border:0;background:none;color:inherit;font:inherit;text-align:left;cursor:pointer;padding:8px 10px;display:flex;flex-direction:column;gap:2px;';
      const t = el('span', null, c.title);
      t.style.cssText = 'font-size:13.5px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;';
      const when = el('span', null, relativeTime(c.updated));
      when.style.cssText = 'font-size:11.5px;color:var(--text-muted);';
      open.append(t, when);
      open.addEventListener('click', () => actions.openChat(c.id));
      const del = el('button', 'chat-item-delete');
      del.setAttribute('aria-label', `Delete ${c.title}`);
      del.style.cssText = 'border:0;background:none;color:var(--text-muted);cursor:pointer;width:26px;height:26px;border-radius:6px;display:flex;align-items:center;justify-content:center;flex:none;opacity:0;';
      del.append(icon('M6 6l12 12M18 6L6 18', 13));
      del.addEventListener('click', (e) => { e.stopPropagation(); actions.deleteChat(c.id); });
      row.addEventListener('mouseenter', () => { del.style.opacity = '1'; });
      row.addEventListener('mouseleave', () => { del.style.opacity = '0'; });
      row.append(open, del);
      m.list.append(row);
    }
  }

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
    }
    m.adopt = null;
    m.lastChat = chatKey;
  }
  if (messages.length < m.rendered) { m.column.replaceChildren(); m.rendered = 0; m.streamNode = null; }
  if (m.streamNode && (messages.length > m.rendered || !state.chatStream)) {
    // the streamed reply became a saved message: replace its live node
    m.streamNode.remove();
    m.streamNode = null;
  }
  const empty = m.column.querySelector('.chat-empty');
  if ((messages.length || state.chatStream) && empty) empty.remove();
  for (let i = m.rendered; i < messages.length; i++) {
    m.column.append(messageNode(messages[i], actions.openLink, m.rendered > 0 || state.chatStream != null || m.animateNext));
  }
  m.rendered = messages.length;
  m.animateNext = false;

  if (!messages.length && !state.chatStream && !m.column.querySelector('.chat-empty')) {
    const e = el('div', 'chat-empty');
    e.style.cssText = 'margin:auto;display:flex;flex-direction:column;align-items:center;gap:16px;text-align:center;padding:40px 0;';
    const stars = thinkingStars();
    stars.querySelector('.think-label').remove();
    stars.classList.add('calm');
    const h = el('div', null, 'How can I help?');
    h.style.cssText = 'font-size:28px;font-weight:600;letter-spacing:-.4px;color:var(--text-strong);';
    const sub = el('div', null, 'Ask anything. I can search the web and open pages for you.');
    sub.style.cssText = 'font-size:14.5px;color:var(--text-muted);';
    const chips = el('div');
    chips.style.cssText = 'display:flex;flex-direction:column;gap:8px;margin-top:8px;';
    for (const s of SUGGESTIONS) {
      const b = el('button', 'chat-suggestion', s);
      b.style.cssText = 'border:0;background:var(--w-field);color:var(--text-soft);font:inherit;font-size:13.5px;padding:9px 16px;border-radius:var(--r-answer);cursor:pointer;';
      b.addEventListener('click', () => actions.send(s));
      chips.append(b);
    }
    e.append(stars, h, sub, chips);
    m.column.append(e);
  }

  // the answer being written
  const stream = state.chatStream;
  if (stream && !stream.done) {
    if (!m.streamNode) {
      m.streamNode = messageNode({ role: 'assistant', text: '' }, actions.openLink, true);
      m.column.append(m.streamNode);
    }
    fillAssistant(m.streamNode.lastChild, stream, actions.openLink, false);
  } else if (stream && stream.done && stream.error && !m.streamNode) {
    m.streamNode = messageNode({ role: 'assistant', text: stream.error }, actions.openLink, true);
    m.column.append(m.streamNode);
  }

  // send / stop
  m.sendBtn.replaceChildren(busy ? icon('M7 7h10v10H7z', 14, true) : icon('M12 19V6m0 0l-5 5m5-5l5 5', 18));
  m.sendBtn.setAttribute('aria-label', busy ? 'Stop' : 'Send');
  m.sendBtn.classList.toggle('busy', busy);
  stayAtBottom();
}

/** Called when a different page replaces the chat page. */
export function chatUnmount() { mounted = null; }
export function chatFocusInput() { mounted?.ta.focus(); }
export function chatAdopt(id) { if (mounted) mounted.adopt = id; }
export function chatAnimateNext() { if (mounted) mounted.animateNext = true; }
