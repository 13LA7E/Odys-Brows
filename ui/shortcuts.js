// Keyboard shortcuts in our own window (AUDIT-41). The list of commands and their keys, and every
// rule about which keys are allowed, live in Rust (app/shell/src/shortcuts.rs); this file only turns
// a key press into a combination like "Ctrl+Shift+K", asks Rust to run the command bound to it, and
// helps the Settings page record a new combination.

let table = [];
let capture = null; // while recording a new shortcut: { done(combo|null) }

const CODE_KEYS = {
  Minus: '-', Equal: '=', Comma: ',', Period: '.', Slash: '/', Semicolon: ';', Quote: "'",
  BracketLeft: '[', BracketRight: ']', Backslash: '\\', Backquote: '`',
  Tab: 'Tab', Enter: 'Enter', Space: 'Space', Escape: 'Esc', Backspace: 'Backspace',
  Delete: 'Delete', Insert: 'Insert', Home: 'Home', End: 'End', PageUp: 'PageUp', PageDown: 'PageDown',
  ArrowLeft: 'Left', ArrowRight: 'Right', ArrowUp: 'Up', ArrowDown: 'Down',
  NumpadAdd: 'Numpad+', NumpadSubtract: 'Numpad-', NumpadMultiply: 'Numpad*',
  NumpadDivide: 'Numpad/', NumpadDecimal: 'Numpad.',
};

/** The key part of a key press by its physical key (so it is the same on every keyboard layout). */
function keyName(e) {
  const c = e.code || '';
  let m;
  if ((m = /^Key([A-Z])$/.exec(c))) return m[1];
  if ((m = /^Digit([0-9])$/.exec(c))) return m[1];
  if ((m = /^Numpad([0-9])$/.exec(c))) return 'Numpad' + m[1];
  if ((m = /^F([0-9]{1,2})$/.exec(c)) && +m[1] >= 1 && +m[1] <= 12) return c;
  if (CODE_KEYS[c]) return CODE_KEYS[c];
  return '';
}

/** "Ctrl+Alt+Shift+K" for a key press, or '' for a lone modifier or an unknown key. */
export function comboFromEvent(e) {
  const key = keyName(e);
  if (!key) return '';
  let s = '';
  if (e.ctrlKey || e.metaKey) s += 'Ctrl+';
  if (e.altKey) s += 'Alt+';
  if (e.shiftKey) s += 'Shift+';
  return s + key;
}

function send(msg) {
  window.odysseyBridge?.(JSON.stringify(msg));
}

/** Replace the list of commands (sent by Rust after every change). */
export function setShortcutTable(list) {
  table = Array.isArray(list) ? list : [];
}

export function getShortcutTable() {
  return table;
}

/** Start recording: the next key combination (or Esc to cancel) is passed to `done`. */
export function captureShortcut(done) {
  capture = { done };
}

export function cancelCapture() {
  if (capture) {
    const c = capture;
    capture = null;
    c.done(null);
  }
}

/** Listen for shortcuts in our own window. Page windows are handled by the engine's key hook. */
export function initShortcuts() {
  window.addEventListener('keydown', (e) => {
    if (capture) {
      if (e.key === 'Control' || e.key === 'Shift' || e.key === 'Alt' || e.key === 'Meta') return;
      e.preventDefault();
      e.stopPropagation();
      const c = capture;
      capture = null;
      c.done(e.key === 'Escape' && !e.ctrlKey && !e.altKey && !e.shiftKey ? null : comboFromEvent(e) || '?');
      return;
    }
    const combo = comboFromEvent(e);
    if (!combo) return;
    const cmd = table.find((c) => c.keys.includes(combo));
    if (!cmd) return;
    e.preventDefault();
    send({ type: 'run-command', id: cmd.id });
  }, true);
}
