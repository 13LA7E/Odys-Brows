// Fixture data (P1-04): the exact sample content of each design board, so `?fixture=<Board>`
// renders the UI with no engine and can be compared with design/png/<Board>.png.
// Star positions and the sample website come from design/preview via tools/extract_fixtures.py.
// Text comes from the boards themselves (DESIGN.md §9).

const TABS_NEWTAB = [
  { id: 1, title: 'Search: celestial navigation' },
  { id: 2, title: 'Reading the sky before maps', parent: 1 },
  { id: 3, title: 'Tide tables' },
  { id: 4, title: 'Lecture notes, week 9' },
  { id: 5, title: 'Train timetable' },
  { id: 6, title: 'Recipe: lentil soup' },
];

const BROWSING_TABS = [
  { id: 1, title: 'Search: celestial navigation' },
  { id: 2, title: 'Reading the sky before maps', parent: 1 },
  { id: 3, title: 'Star chart (PDF)', parent: 1 },
  { id: 4, title: 'Tide tables' },
  { id: 5, title: 'Lecture notes, week 9' },
];

const ARTICLE_ADDRESS = { kind: 'web', host: 'northstar.example', path: '/essays/reading-the-sky' };

export const FIXTURES = {
  Main: {
    rail: 'collapsed',
    tabs: TABS_NEWTAB,
    activeTab: 1,
    page: 'none',
    newTab: {
      greeting: 'Good evening',
      clock: '20:11',
      date: 'Wednesday, 18 November',
      hint: 'Your open tabs, drawn as a trail. Select a star to jump back to it.',
      // Illustrative layout from the board (DESIGN.md §9); the real one is generated (P3-01).
      points: [[520, 330], [668, 236], [806, 352], [960, 262], [888, 508], [700, 560]],
      edges: [[0, 1], [1, 2], [2, 3], [2, 4], [4, 5]],
      labels: ['New tab', 'Tide tables', 'Lecture notes, week 9', 'Train timetable', 'Recipe: lentil soup', 'Search: celestial navigation'],
      current: 0,
    },
    helm: { kind: 'empty' },
  },
  Browsing: {
    rail: 'expanded',
    tabs: BROWSING_TABS,
    activeTab: 2,
    page: 'web',
    helm: { kind: 'address', address: ARTICLE_ADDRESS, download: 0.62 },
    downloads: [
      { name: 'northstar-star-chart.pdf', detail: '124 MB of 200 MB, 11.2 MB/s, 8 connections', progress: 0.62, active: true },
      { name: 'tide-tables-2026.pdf', detail: 'Done', done: true },
    ],
    downloadsOpen: true,
  },
  Settings: {
    rail: 'collapsed',
    tabCount: 4,
    activeTab: 2,
    page: 'internal',
    helm: { kind: 'address', address: { kind: 'internal', name: 'settings' } },
    settings: {
      nav: ['General', 'Appearance', 'New tab', 'Shields', 'Odysseus', 'Passwords', 'Privacy', 'Shortcuts', 'About'],
      section: 'Appearance',
      stars: 'Still', shootingStars: false, accent: 'Sea', trail: 'Narrow', connections: 8,
    },
  },
  HelmAnswer: {
    rail: 'collapsed',
    tabCount: 5,
    activeTab: 2,
    page: 'web',
    dimPage: true,
    helm: { kind: 'asking', question: 'What did navigators use when the guide star was hidden?' },
    answer: {
      text: 'They fell back on other guide stars that rose or set near the same point on the horizon, and on swell direction and wind. The essay describes it as reading the same sky in a different order.',
      sources: ['northstar.example', 'Star chart (PDF)'],
      footer: 'Answered by your Odysseus server',
      action: 'Continue in Odysseus',
    },
  },
  ClosedTabs: {
    rail: 'expanded',
    tabs: [
      { id: 1, title: 'Search: celestial navigation' },
      { id: 2, title: 'Reading the sky before maps', parent: 1 },
      { id: 3, title: 'Star chart (PDF)', parent: 1 },
      { id: 5, title: 'Lecture notes, week 9' },
    ],
    activeTab: 2,
    closed: [
      { title: 'Train timetable' },
      { title: 'Tide tables', host: 'tides.example', closed: 'Closed 12 minutes ago', hovered: true },
      { title: 'Recipe: lentil soup' },
    ],
    page: 'web',
    helm: { kind: 'address', address: ARTICLE_ADDRESS },
  },
  Private: {
    rail: 'private',
    tabCount: 2,
    activeTab: 1,
    page: 'none',
    private: true,
    privateCopy: {
      title: 'Private voyage',
      body: 'This window leaves no trail behind. Nothing you open here is kept once you close it.',
      bullets: ['History is not saved', 'Cookies and site data are cleared on close', 'Shields stay on'],
    },
    helm: { kind: 'empty' },
  },
  History: {
    rail: 'collapsed',
    tabCount: 5,
    activeTab: 5,
    page: 'internal',
    helm: { kind: 'address', address: { kind: 'internal', name: 'history' } },
    history: {
      search: 'Search your trail',
      days: [
        { label: 'Today', rows: [
          { time: '20:04', title: 'Reading the sky before maps', host: 'northstar.example', current: true, branch: true },
          { time: '19:58', title: 'Search: celestial navigation', host: 'search.example' },
          { time: '19:41', title: 'Tide tables', host: 'tides.example', cameBack: true, hovered: true },
          { time: '19:22', title: 'Lecture notes, week 9', host: 'campus.example' },
        ] },
        { label: 'Yesterday', rows: [
          { time: '22:10', title: 'Train timetable', host: 'rail.example' },
          { time: '21:47', title: 'Recipe: lentil soup', host: 'cooking.example' },
          { time: '21:30', title: 'Orbital mechanics, lecture 4', host: 'campus.example' },
          { time: '20:55', title: 'Tide tables', host: 'tides.example' },
        ] },
      ],
      side: { title: 'Today', summary: 'Four pages, one branch. The bright star is the page you came back to.' },
    },
  },
};

/** Shields count shown in the notch on every board. */
export const SHIELDS_COUNT = 12;
