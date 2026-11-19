// Browser chrome entry point. Product strings live in brand.js (DESIGN.md section 11).
import { NAME } from './brand.js';
import './bridge.js';

document.title = NAME;

const params = new URLSearchParams(location.search);
const fixture = params.get('fixture');

if (fixture) {
  // Fixture mode (P1-04): sample content, no engine.
  import('./fixture.js').then((m) => m.startFixture(fixture));
} else if (location.hash === '#ipc-selftest') {
  import('./selftest.js');
}
