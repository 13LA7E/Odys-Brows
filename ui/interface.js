// Interface options (AUDIT-39): corner roundness, the size of the whole interface, which Helm
// buttons show, and how the answer card reads. Everything is a CSS variable or an attribute on
// the app root, so the default (nothing set) is exactly the approved design. Not offered:
// tab style and position, rail side, window blur and opacity, density (D-025).

const BASE_RADII = {
  '--r-helm': 26, '--r-answer': 20, '--r-popover': 14, '--r-search': 12,
  '--r-track': 10, '--r-row': 8, '--r-chip': 8, '--r-segment': 7,
};
const RADIUS_FACTOR = { Sharp: 0.15, Default: 1, Round: 1.5 };
const FONTS = {
  Sans: null,
  Serif: 'Cambria, Georgia, "Times New Roman", serif',
  Mono: '"Cascadia Code", Consolas, "Courier New", monospace',
};
const SIZES = { Small: 14, Default: 15.5, Large: 17.5 };
const WIDTHS = { Narrow: 480, Default: 560, Wide: 680 };
const SCALES = { '90%': 0.9, '100%': 1, '110%': 1.1, '125%': 1.25 };

let lastKey = '';
let scale = 1;

/** The interface size factor now in use (1 = design size). */
export function interfaceScale() { return scale; }

/** Applies the saved interface options (cheap when nothing changed). `root` is the #app element. */
export function applyInterface(settings, root) {
  const s = settings || {};
  const key = JSON.stringify([s.reduceMotion, s.radius, s.uiScale, s.showNavButtons, s.showReload, s.showGuideBar, s.answerFont, s.answerSize, s.answerWidth, window.innerWidth, window.innerHeight]);
  if (key === lastKey) return;
  lastKey = key;
  const docEl = document.documentElement;
  const set = (name, value) => (value == null ? docEl.style.removeProperty(name) : docEl.style.setProperty(name, value));

  const f = RADIUS_FACTOR[s.radius] ?? 1;
  for (const [name, px] of Object.entries(BASE_RADII)) set(name, f === 1 ? null : `${Math.round(px * f)}px`);

  set('--answer-font', FONTS[s.answerFont] || null);
  const size = SIZES[s.answerSize];
  set('--answer-size', size && size !== SIZES.Default ? `${size}px` : null);
  const width = WIDTHS[s.answerWidth];
  set('--answer-w', width && width !== WIDTHS.Default ? `${width}px` : null);

  if (s.reduceMotion === 'On') docEl.dataset.reduce = 'on';
  else if (s.reduceMotion === 'Off') docEl.dataset.reduce = 'off';
  else delete docEl.dataset.reduce;

  if (root) {
    root.dataset.nav = s.showNavButtons === false ? 'off' : 'on';
    root.dataset.reload = s.showReload === false ? 'off' : 'on';
    root.dataset.guide = s.showGuideBar === false ? 'off' : 'on';
    // Scale the whole interface: the app is laid out as if the window were smaller or bigger.
    const z = document.documentElement.classList.contains('fixture') ? 1 : (SCALES[s.uiScale] || 1);
    scale = z;
    if (z === 1 || document.documentElement.classList.contains('fixture')) {
      root.style.removeProperty('zoom');
      root.style.removeProperty('width');
      root.style.removeProperty('height');
    } else {
      root.style.zoom = String(z);
      root.style.width = `${window.innerWidth / z}px`;
      root.style.height = `${window.innerHeight / z}px`;
    }
  }
}
