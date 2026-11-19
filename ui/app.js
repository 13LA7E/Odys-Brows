// Browser chrome. Product strings live in brand.js (DESIGN.md section 11).
import { NAME } from './brand.js';
import './bridge.js';

document.title = NAME;

if (location.hash === '#ipc-selftest') {
  import('./selftest.js');
}
