// The UI side of the message bridge (P1-03). Rust defines window.odysseyBridge only in this
// page; web pages never have it. Messages are JSON objects with a `type` field
// (see app/shell/src/bridge.rs for the list).

const listeners = new Set();

// Rust calls this with one message object.
window.odysseyReceive = (message) => {
  for (const listener of listeners) {
    try { listener(message); } catch (e) { console.error('bridge listener failed', e); }
  }
};

/** True when the native side is reachable (false in fixture mode in a normal browser). */
export function hasBridge() {
  return typeof window.odysseyBridge === 'function';
}

/** Send one message to Rust. Returns false when there is no bridge. */
export function send(message) {
  if (!hasBridge()) return false;
  window.odysseyBridge(JSON.stringify(message));
  return true;
}

/** Listen for messages from Rust. Returns a function that stops listening. */
export function onMessage(listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
