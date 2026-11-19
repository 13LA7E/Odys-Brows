// `#ipc-selftest` (P1-03): from the UI, open a page view, navigate it and see its title come
// back; then open a probe page and check it cannot see the bridge. Reports with
// {type: 'selftest-result'}. Loaded only when the URL fragment asks for it.
import { hasBridge, onMessage, send } from './bridge.js';

const PAGE_A = 'data:text/html,<title>Page A</title><p>A';
const PAGE_B = 'data:text/html,<title>Page B</title><p>B';
// A normal page reports what it can see of our bridge in its own title.
const PROBE = 'data:text/html,<script>document.title = typeof window.odysseyBridge + "," + typeof window.odysseyReceive;</script>';

const log = [];

function waitFor(test, ms = 15000) {
  return new Promise((resolve, reject) => {
    const stop = onMessage((m) => { if (test(m)) { stop(); clearTimeout(timer); resolve(m); } });
    const timer = setTimeout(() => { stop(); reject(new Error('timed out')); }, ms);
  });
}

async function run() {
  if (!hasBridge()) throw new Error('our UI has no bridge');
  log.push('UI has the bridge');

  send({ type: 'open', url: PAGE_A });
  const { view } = await waitFor((m) => m.type === 'view-opened');
  await waitFor((m) => m.type === 'title' && m.view === view && m.title === 'Page A');
  log.push(`view ${view}: title "Page A" came back`);

  send({ type: 'navigate', view, url: PAGE_B });
  await waitFor((m) => m.type === 'title' && m.view === view && m.title === 'Page B');
  log.push(`view ${view}: navigated, title "Page B" came back`);

  send({ type: 'open', url: PROBE });
  const probe = (await waitFor((m) => m.type === 'view-opened')).view;
  const seen = await waitFor((m) => m.type === 'title' && m.view === probe && m.title.includes(','));
  if (seen.title !== 'undefined,undefined') throw new Error(`a web page can see the bridge: ${seen.title}`);
  log.push(`probe page sees: ${seen.title} (no bridge)`);
}

run().then(
  () => send({ type: 'selftest-result', ok: true, log }),
  (e) => { log.push(String(e && e.message || e)); send({ type: 'selftest-result', ok: false, log }); },
);
