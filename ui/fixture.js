// Fixture mode (P1-04): `index.html?fixture=<Board>` renders that board's sample content with
// no engine, at 1280x800, for tools/visual_check.py. P1-04 draws the frame every board shares
// (sky, rail area, page panel, Helm frame, notch); Phase 2 tasks fill in the parts.
import { FIXTURES, SHIELDS_COUNT } from './fixtures.js';
import { renderShell } from './shell.js';

export async function startFixture(board) {
  const data = FIXTURES[board];
  const app = document.getElementById('app');
  if (!data) {
    app.textContent = `Unknown fixture "${board}". Known: ${Object.keys(FIXTURES).join(', ')}.`;
    app.className = 'fixture-error';
    return;
  }
  document.documentElement.classList.add('fixture');
  document.documentElement.dataset.motion = 'off'; // keeps visual_check stable
  const sky = await (await fetch('fixtures/sky.json')).json();
  const page = data.page === 'web'
    ? await (await fetch(`fixtures/page-${board}.html`)).text()
    : '';
  renderShell(app, { ...data, sky: sky[board] || [], pageHtml: page, shields: SHIELDS_COUNT });
}
