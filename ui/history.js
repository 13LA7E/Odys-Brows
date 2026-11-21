// History page (DESIGN.md §9, PHASES.md P4-03).
// Renders the Trail-shaped history with day groups, branches, came-back stars,
// search input, hover actions (Open / Forget), and side panel mini-constellation.

const SVG = 'http://www.w3.org/2000/svg';

function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}

function svg(tag, attrs) {
  const e = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  return e;
}

let searchOpen = false;

const STAR_PATH = 'M12 2C12.8 8.5 15.5 11.2 22 12C15.5 12.8 12.8 15.5 12 22C11.2 15.5 8.5 12.8 2 12C8.5 11.2 11.2 8.5 12 2Z';

/** Convert live DayGroups from Rust bridge into rows for renderHistory */
export function formatDayGroups(groups = []) {
  return groups.map((g) => ({
    label: g.label,
    day_start_secs: g.day_start_secs,
    rows: (g.visits || []).map((v) => {
      const d = new Date(v.timestamp * 1000);
      const hh = String(d.getHours()).padStart(2, '0');
      const mm = String(d.getMinutes()).padStart(2, '0');
      return {
        id: v.id,
        time: `${hh}:${mm}`,
        title: v.title || v.host || v.url,
        host: v.host,
        url: v.url,
        cameBack: !!v.came_back,
        branch: v.opener_id != null,
      };
    }),
  }));
}

/**
 * Render history view inside the page panel.
 *
 * @param {HTMLElement} container - The panel container (.page-internal)
 * @param {object} historyState - History state with { days, search, side }
 * @param {object} callbacks - { onSearch, onOpen, onForget, onClearDay }
 */
export function renderHistory(container, historyState = {}, callbacks = {}) {
  container.replaceChildren();

  const days = historyState.days || [];
  const searchPlaceholder = historyState.search || 'Search your trail';
  const side = historyState.side || {
    title: 'Today',
    summary: 'Four pages, one branch. The bright star is the page you came back to.',
  };

  // 1. Heading "History" (32/600) at left 120, top 56
  const title = el('div', 'history-title', 'History');
  title.style.cssText = 'position:absolute;left:120px;top:56px;font-size:32px;font-weight:600;letter-spacing:-.5px;';
  container.append(title);

  // 2. Search field at left 120, top 118, 660x46, radius 12, var(--w-field)
  const searchBar = el('div', 'history-search');
  searchBar.style.cssText = 'position:absolute;left:120px;top:118px;width:660px;height:46px;border-radius:12px;background:var(--w-field);display:flex;align-items:center;gap:12px;padding:0 16px;box-sizing:border-box;font-size:15px;color:var(--text-muted);';

  const searchIconSvg = svg('svg', {
    width: '18',
    height: '18',
    viewBox: '0 0 24 24',
    fill: 'none',
    stroke: 'var(--text-muted)',
    'stroke-width': '1.6',
    'stroke-linecap': 'round',
    'stroke-linejoin': 'round',
    'aria-hidden': 'true',
  });
  searchIconSvg.append(
    svg('circle', { cx: '11', cy: '11', r: '6' }),
    svg('path', { d: 'M16 16l4 4' }),
  );

  const searchSpan = el('span', 'history-search-placeholder', searchPlaceholder);
  searchSpan.style.cssText = 'cursor:text;flex:1;';
  searchBar.append(searchIconSvg, searchSpan);
  const openSearchInput = () => {
    searchOpen = true;
    const input = el('input');
    input.type = 'text';
    input.value = historyState.searchQuery || '';
    input.placeholder = searchPlaceholder;
    input.style.cssText = 'border:0;background:none;color:var(--text);font:inherit;font-size:15px;outline:none;flex:1;padding:0;margin:0;';
    input.addEventListener('input', (e) => callbacks.onSearch?.(e.target.value));
    searchBar.replaceChild(input, searchBar.querySelector('.history-search-placeholder') || input);
    input.focus();
  };
  searchBar.addEventListener('click', () => {
    if (!searchBar.querySelector('input')) openSearchInput();
  });
  // A re-render (new results arrive) must not close the field the user is typing in.
  if (searchOpen && historyState.searchQuery != null) openSearchInput();
  container.append(searchBar);

  // 3. Calculate layout coordinates for day groups and rows
  let currentGroupTop = 206;
  const renderedGroups = [];

  days.forEach((group) => {
    const groupHeaderTop = currentGroupTop;
    const firstRowCenterY = groupHeaderTop + 56;
    const rowPositions = [];

    (group.rows || []).forEach((row, rowIdx) => {
      const centerY = firstRowCenterY + rowIdx * 44;
      const topY = centerY - 19; // 38px height centered at centerY
      rowPositions.push({ row, centerY, topY });
    });

    renderedGroups.push({
      group,
      headerTop: groupHeaderTop,
      rowPositions,
    });

    const lastCenterY = rowPositions.length > 0
      ? rowPositions[rowPositions.length - 1].centerY
      : firstRowCenterY;
    currentGroupTop = lastCenterY + 52;
  });

  // 4. SVG Trail Overlay
  const trailSvg = svg('svg', {
    width: '1224',
    height: '800',
    viewBox: '0 0 1224 800',
    style: 'position:absolute;left:0;top:0;',
    'aria-hidden': 'true',
  });

  renderedGroups.forEach(({ rowPositions }) => {
    if (rowPositions.length === 0) return;

    const spineRows = rowPositions.filter((p) => !p.row.branch);
    if (spineRows.length > 1) {
      const minY = spineRows[0].centerY;
      const maxY = spineRows[spineRows.length - 1].centerY;
      trailSvg.append(svg('line', {
        x1: '186',
        y1: String(minY),
        x2: '186',
        y2: String(maxY),
        stroke: 'var(--white)',
        'stroke-opacity': '.16',
      }));
    }

    rowPositions.forEach(({ row, centerY }) => {
      if (row.branch) {
        // Curved branch from trunk below (x 186, y 306) to branch row (x 206, y 262)
        const trunkY = centerY + 44;
        trailSvg.append(svg('path', {
          d: `M186 ${trunkY} V${centerY + 8} Q186 ${centerY} 194 ${centerY} H206`,
          fill: 'none',
          stroke: 'var(--white)',
          'stroke-opacity': '.16',
        }));

        if (row.current) {
          trailSvg.append(svg('circle', {
            cx: '206',
            cy: String(centerY),
            r: '11',
            fill: 'var(--panel-internal)',
            stroke: 'var(--accent)',
            'stroke-opacity': '.4',
          }));
          const starSvg = svg('svg', {
            x: '199',
            y: String(centerY - 7),
            width: '14',
            height: '14',
            viewBox: '0 0 24 24',
          });
          starSvg.append(svg('path', {
            d: STAR_PATH,
            fill: 'var(--accent)',
          }));
          trailSvg.append(starSvg);
        } else {
          trailSvg.append(svg('circle', {
            cx: '206',
            cy: String(centerY),
            r: '3.4',
            fill: 'var(--node)',
          }));
        }
      } else if (row.cameBack) {
        trailSvg.append(svg('circle', {
          cx: '186',
          cy: String(centerY),
          r: '5',
          fill: 'var(--white)',
          opacity: '.95',
        }));
      } else if (row.hovered) {
        trailSvg.append(svg('circle', {
          cx: '186',
          cy: String(centerY),
          r: '3.4',
          fill: 'var(--node)',
        }));
        trailSvg.append(svg('circle', {
          cx: '186',
          cy: String(centerY),
          r: '3.4',
          fill: 'var(--white)',
        }));
      } else {
        trailSvg.append(svg('circle', {
          cx: '186',
          cy: String(centerY),
          r: '3.4',
          fill: 'var(--node)',
        }));
      }
    });
  });

  // Append trail SVG
  container.append(trailSvg);

  // 5. Render Day headers and rows
  renderedGroups.forEach(({ group, headerTop, rowPositions }) => {
    const header = el('div', 'history-group-header');
    header.style.cssText = `position:absolute;left:120px;top:${headerTop}px;width:660px;display:flex;justify-content:space-between;align-items:baseline;`;

    const label = el('span', null, group.label);
    label.style.cssText = 'font-size:15px;font-weight:600;';

    const clearBtn = el('span', null, 'Clear this day');
    clearBtn.style.cssText = 'font-size:13px;color:var(--text-muted);cursor:pointer;';
    clearBtn.addEventListener('click', () => {
      callbacks.onClearDay?.(group.day_start_secs);
    });

    header.append(label, clearBtn);
    container.append(header);

    rowPositions.forEach(({ row, topY }) => {
      const rowDiv = el('div', 'history-row');
      const isHovered = !!row.hovered;
      const isCurrent = !!row.current;
      const isCameBack = !!row.cameBack;

      rowDiv.style.cssText = `position:absolute;left:108px;top:${topY}px;width:660px;height:38px;border-radius:8px;${isHovered ? 'background:var(--w-hover-row);' : ''}display:flex;align-items:center;font-size:14.5px;box-sizing:border-box;`;

      const timeSpan = el('span', null, row.time);
      timeSpan.style.cssText = 'width:64px;font-size:13px;color:var(--text-muted);font-variant-numeric:tabular-nums;';

      const titleMarginLeft = row.branch ? '64px' : '44px';
      const titleColor = (isCurrent || isHovered || isCameBack) ? 'var(--text)' : 'var(--text-soft)';

      const titleSpan = el('span', null, row.title);
      titleSpan.style.cssText = `margin-left:${titleMarginLeft};color:${titleColor};white-space:nowrap;`;

      const hostSpan = el('span', null, row.host);
      hostSpan.style.cssText = 'margin-left:10px;font-size:13px;color:var(--text-muted);white-space:nowrap;';

      rowDiv.append(timeSpan, titleSpan, hostSpan);

      // Open / Forget: shown for the design's pre-hovered row, and on real hover (CSS) in the live UI.
      const actions = el('span', 'history-actions');
      actions.style.cssText = 'margin-left:auto;gap:16px;font-size:13px;';
      if (isHovered) rowDiv.classList.add('is-hovered');

      const openBtn = el('span', null, 'Open');
      openBtn.style.cssText = 'color:var(--accent);cursor:pointer;';
      openBtn.addEventListener('click', () => {
        callbacks.onOpen?.(row.url || `https://${row.host}`);
      });

      const forgetBtn = el('span', null, 'Forget');
      forgetBtn.style.cssText = 'color:var(--text-soft);cursor:pointer;';
      forgetBtn.addEventListener('click', () => {
        callbacks.onForget?.(row.id);
      });

      actions.append(openBtn, forgetBtn);
      const spacer = el('span');
      spacer.style.cssText = 'width:12px;';
      rowDiv.append(actions, spacer);

      container.append(rowDiv);
    });
  });

  // 6. Side panel at left 880, top 236, width 300
  if (side) {
    const sideDiv = el('div', 'history-side-panel');
    sideDiv.style.cssText = 'position:absolute;left:880px;top:236px;width:300px;';

    const sideTitle = el('div', null, side.title || 'Today');
    sideTitle.style.cssText = 'font-size:15px;font-weight:600;';

    const sideSummary = el('div', null, side.summary || 'Four pages, one branch. The bright star is the page you came back to.');
    sideSummary.style.cssText = 'font-size:13px;color:var(--text-muted);margin-top:4px;line-height:1.5;';

    const constelWrap = el('div');
    constelWrap.style.cssText = 'margin-top:18px;';

    const constelSvg = svg('svg', {
      width: '300',
      height: '170',
      viewBox: '0 0 300 170',
      'aria-hidden': 'true',
    });

    const lines = [
      [40, 120, 96, 60],
      [96, 60, 150, 104],
      [150, 104, 206, 44],
      [150, 104, 254, 92],
    ];
    lines.forEach(([x1, y1, x2, y2]) => {
      constelSvg.append(svg('line', {
        x1: String(x1),
        y1: String(y1),
        x2: String(x2),
        y2: String(y2),
        stroke: 'var(--white)',
        'stroke-opacity': '.18',
      }));
    });

    const points = [
      [40, 120, 3],
      [96, 60, 3],
      [150, 104, 4.6],
      [206, 44, 3],
      [254, 92, 3],
    ];
    points.forEach(([cx, cy, r]) => {
      constelSvg.append(svg('circle', {
        cx: String(cx),
        cy: String(cy),
        r: String(r),
        fill: 'var(--white)',
        opacity: '.9',
      }));
    });

    // First point ring and accent star
    constelSvg.append(svg('circle', {
      cx: '40',
      cy: '120',
      r: '12',
      fill: 'none',
      stroke: 'var(--accent)',
      'stroke-opacity': '.4',
    }));

    const starG = svg('svg', {
      x: '30',
      y: '110',
      width: '20',
      height: '20',
      viewBox: '0 0 24 24',
    });
    starG.append(svg('path', {
      d: STAR_PATH,
      fill: 'var(--accent)',
    }));
    constelSvg.append(starG);

    constelWrap.append(constelSvg);
    sideDiv.append(sideTitle, sideSummary, constelWrap);
    container.append(sideDiv);
  }
}
