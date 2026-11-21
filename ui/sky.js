// Sky rendering on canvas (DESIGN.md §5, PHASES.md P2-01).
// Still mode renders once at device pixel ratio (0% idle CPU).
// Drifting and Shooting Stars (comets) run slow, subtle animations when enabled,
// paused immediately whenever the window or tab is hidden or when reduced motion is preferred.

// The drift is a few pixels per second, so while nothing is going on the sky repaints only this
// often; redrawing the whole window 60 times a second for it cost most of a CPU core in the GPU
// process. It speeds up (still capped) while the pointer moves or a comet crosses.
const SKY_IDLE_FRAME_MS = 200; // nothing going on: 5 frames a second (the drift is < 1 px per step)
const SKY_ACTIVE_FRAME_MS = 33; // pointer or comet: 30 frames a second
const SKY_ACTIVE_WINDOW_MS = 1500;

// Every sky that exists, so a test can check that only one animation loop runs (AUDIT-04).
const liveSkies = new Set();

/** Number of sky animation loops currently running. */
export function skyLoopCount() {
  let n = 0;
  for (const s of liveSkies) if (s.isRunning()) n += 1;
  return n;
}
if (typeof window !== 'undefined') window.__skyLoopCount = skyLoopCount;

export function createSky(container, options = {}) {
  const canvas = document.createElement('canvas');
  canvas.className = 'sky';
  canvas.setAttribute('aria-hidden', 'true');

  const isFixture = document.body.classList.contains('fixture');
  const width = options.width || (isFixture ? 1280 : (window.innerWidth || 1280));
  const height = options.height || (isFixture ? 800 : (window.innerHeight || 800));
  const mode = options.mode || 'still'; // 'off' | 'still' | 'drifting'
  const enableComets = !!options.shootingStars;
  const isPrivate = !!options.private;

  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.round(width * dpr);
  canvas.height = Math.round(height * dpr);
  canvas.style.width = `${width}px`;
  canvas.style.height = `${height}px`;

  const ctx = canvas.getContext('2d');
  ctx.scale(dpr, dpr);

  let stars = options.stars ? options.stars.map((s) => ({ ...s })) : [];
  const maxStarX = stars.length > 0 ? Math.max(...stars.map((s) => s.x)) : 0;
  if (!options.stars && mode !== 'off') {
    stars = generateSky(options.type || (isPrivate ? 'private' : 'newtab'), width, height);
  } else if (!isFixture && mode !== 'off' && (width > 1280 || height > 800) && maxStarX <= 1280) {
    const baseCount = stars.length || 170;
    const extraCount = Math.round(((width * height - 1280 * 800) / (1280 * 800)) * baseCount);
    if (extraCount > 0) {
      const radii = [0.6, 0.7, 0.8, 0.9, 1.1, 1.3];
      const opacities = [0.25, 0.35, 0.45, 0.55, 0.7];
      let seed = 12345;
      function rand() {
        seed = (seed * 16807) % 2147483647;
        return (seed - 1) / 2147483646;
      }
      for (let i = 0; i < extraCount; i++) {
        let x, y;
        const rightArea = (width - 1280) * height;
        const totalExtraArea = width * height - 1280 * 800;
        if (rand() < rightArea / totalExtraArea) {
          x = 1280 + rand() * (width - 1280);
          y = rand() * height;
        } else {
          x = rand() * 1280;
          y = 800 + rand() * (height - 800);
        }
        stars.push({
          x,
          y,
          r: radii[Math.floor(rand() * radii.length)],
          o: opacities[Math.floor(rand() * opacities.length)],
          fill: isPrivate ? '#cdd9ee' : '#ffffff',
        });
      }
    }
  }

  // Star density option (AUDIT-36): keep a fixed share of the stars (100% = all, the design).
  const keep = { '25%': 25, '50%': 50, '75%': 75 }[options.density];
  if (keep) stars = stars.filter((_, i) => (i * 37) % 100 < keep);
  const driftScale = { Slow: 0.5, Normal: 1, Fast: 2 }[options.speed] || 1;

  // Pre-calculate deterministic drift velocities (very slow: 0.02 - 0.06 px/frame)
  for (let i = 0; i < stars.length; i++) {
    const angle = ((i * 47) % 360) * (Math.PI / 180);
    const speed = 0.02 + ((i * 13) % 4) * 0.01;
    stars[i].vx = Math.cos(angle) * speed * driftScale;
    stars[i].vy = Math.sin(angle) * speed * driftScale;
  }

  // Stars have no depth in our data; a bigger star counts as nearer (Odyssey's `z`).
  for (const st of stars) st.z = Math.max(0, Math.min(1, ((st.r || 1) - 0.5) / 0.9));

  // Cursor-linked stars (AUDIT-32, ported from Odyssey ui/fx.js, same numbers; no click effect). Only while the
  // sky drifts, never in fixture mode, never with reduced motion. Nothing is allocated per frame.
  const cursorOpt = options.cursor !== false && !isFixture;
  const mouse = { x: -1e4, y: -1e4, tx: 0, ty: 0, sx: 0, sy: 0, inside: false };
  const nearX = new Float32Array(9);
  const nearY = new Float32Array(9);
  const nearD = new Float32Array(9);
  const accentColor = (getComputedStyle(document.documentElement).getPropertyValue('--accent') || '#ffffff').trim();

  let rafId = null;
  let lastTime = 0;
  let waitTimer = 0;
  // How often comets cross: Rare = half as often, Often = twice as often (default 15 to 30 s apart).
  const cometScale = { Rare: 2, Normal: 1, Often: 0.5 }[options.cometRate] || 1;
  const cometGap = () => (15000 + Math.random() * 15000) * cometScale;
  let nextCometTime = performance.now() + cometGap();
  let currentComet = null;
  let isDestroyed = false;
  let paused = false;
  let idle = false;
  let lastActivity = performance.now();

  // Reduce motion (AUDIT-42): the setting wins; Auto follows the system.
  const systemReduce = window.matchMedia('(prefers-reduced-motion: reduce)');
  const prefersReducedMotion = {
    get matches() {
      const r = document.documentElement.dataset.reduce;
      return r === 'on' || (r !== 'off' && systemReduce.matches);
    },
  };

  function drawStar(st, x = st.x, y = st.y, o = st.o) {
    ctx.beginPath();
    ctx.arc(x, y, st.r, 0, Math.PI * 2);
    if (st.ring) {
      ctx.strokeStyle = st.stroke || '#cdd9ee';
      ctx.lineWidth = st.w || 1.0;
      ctx.globalAlpha = o;
      ctx.stroke();
    } else {
      ctx.fillStyle = (options.starColor && st.fill === '#ffffff') ? options.starColor : (st.fill || (isPrivate ? '#cdd9ee' : '#ffffff'));
      ctx.globalAlpha = o;
      ctx.fill();
    }
  }

  function drawComet(comet) {
    if (!comet) return;
    const tailX = comet.x - Math.cos(comet.angle) * comet.length;
    const tailY = comet.y - Math.sin(comet.angle) * comet.length;

    const grad = ctx.createLinearGradient(tailX, tailY, comet.x, comet.y);
    const alpha = comet.alpha;
    grad.addColorStop(0, 'rgba(255, 255, 255, 0)');
    grad.addColorStop(1, `rgba(255, 255, 255, ${alpha.toFixed(3)})`);

    ctx.beginPath();
    ctx.moveTo(tailX, tailY);
    ctx.lineTo(comet.x, comet.y);
    ctx.strokeStyle = grad;
    ctx.lineWidth = 1.2;
    ctx.lineCap = 'round';
    ctx.stroke();
  }

  function renderFrame(now) {
    if (isDestroyed) return;
    if (document.hidden || paused || idle) {
      rafId = null;
      return;
    }

    ctx.clearRect(0, 0, width, height);

    if (mode === 'off') {
      rafId = null;
      return;
    }

    // Still mode: draw once and stop loop (0% idle CPU)
    if (mode === 'still' && (!enableComets || prefersReducedMotion.matches)) {
      for (const st of stars) drawStar(st);
      ctx.globalAlpha = 1.0;
      rafId = null;
      return;
    }

    // Drifting mode or active comet
    const dt = lastTime ? Math.min((now - lastTime) / 1000, 0.1) : 0.016;
    lastTime = now;

    const canAnimate = !prefersReducedMotion.matches;
    const useCursor = cursorOpt && mode === 'drifting' && canAnimate;
    let nearCount = 0;
    if (useCursor) {
      const k = Math.min(1, dt * 3);
      mouse.sx += (mouse.tx - mouse.sx) * k;
      mouse.sy += (mouse.ty - mouse.sy) * k;
    }

    for (const st of stars) {
      if (mode === 'drifting' && canAnimate) {
        st.x += st.vx * (dt / 0.016);
        st.y += st.vy * (dt / 0.016);
        if (st.x < 0) st.x += width;
        if (st.x > width) st.x -= width;
        if (st.y < 0) st.y += height;
        if (st.y > height) st.y -= height;
      }
      if (!useCursor) {
        drawStar(st);
        continue;
      }
      // Parallax against the cursor, scaled by depth: 16 px across, 11 px down at most.
      const x = st.x - mouse.sx * st.z * 16;
      const y = st.y - mouse.sy * st.z * 11;
      let o = st.o;
      if (mouse.inside && st.z > 0.4) {
        const dx = x - mouse.x;
        const dy = y - mouse.y;
        const d2 = dx * dx + dy * dy;
        if (d2 < 22500) {
          o = Math.min(1, o + (1 - Math.sqrt(d2) / 150) * 0.6);
          if (nearCount < 9) {
            nearX[nearCount] = x;
            nearY[nearCount] = y;
            nearD[nearCount] = d2;
            nearCount += 1;
          }
        }
      }
      drawStar(st, x, y, o);
    }

    if (useCursor && nearCount > 1) {
      let nearest = 0;
      for (let i = 1; i < nearCount; i++) if (nearD[i] < nearD[nearest]) nearest = i;
      ctx.lineWidth = 0.7;
      for (let i = 0; i < nearCount; i++) {
        ctx.strokeStyle = i === nearest ? accentColor : '#ffffff';
        ctx.globalAlpha = (1 - Math.sqrt(nearD[i]) / 150) * 0.4;
        ctx.beginPath();
        ctx.moveTo(mouse.x, mouse.y);
        ctx.lineTo(nearX[i], nearY[i]);
        ctx.stroke();
      }
      ctx.strokeStyle = '#ffffff';
      for (let i = 0; i < nearCount; i++) {
        for (let j = i + 1; j < nearCount; j++) {
          const d = Math.hypot(nearX[i] - nearX[j], nearY[i] - nearY[j]);
          if (d < 85) {
            ctx.globalAlpha = (1 - d / 85) * 0.22;
            ctx.beginPath();
            ctx.moveTo(nearX[i], nearY[i]);
            ctx.lineTo(nearX[j], nearY[j]);
            ctx.stroke();
          }
        }
      }
    }

    // Shooting stars (rare, subtle comets across new tab)
    if (enableComets && canAnimate) {
      if (!currentComet && now >= nextCometTime) {
        const startX = 200 + Math.random() * Math.max(1, width - 400);
        const startY = 50 + Math.random() * 200;
        const angle = (25 + Math.random() * 20) * (Math.PI / 180); // gentle downward slant
        currentComet = { x: startX, y: startY, angle, length: 50, speed: 380, distance: 0, maxDistance: 280, alpha: 0 };
      }
      if (currentComet) {
        currentComet.distance += currentComet.speed * dt;
        currentComet.x += Math.cos(currentComet.angle) * currentComet.speed * dt;
        currentComet.y += Math.sin(currentComet.angle) * currentComet.speed * dt;
        const frac = currentComet.distance / currentComet.maxDistance;
        if (frac < 0.2) currentComet.alpha = (frac / 0.2) * 0.45;
        else if (frac > 0.7) currentComet.alpha = (1 - (frac - 0.7) / 0.3) * 0.45;
        else currentComet.alpha = 0.45;
        drawComet(currentComet);
        if (frac >= 1) {
          currentComet = null;
          nextCometTime = now + cometGap();
        }
      }
    }

    ctx.globalAlpha = 1.0;

    if (mode === 'drifting' || (enableComets && canAnimate)) {
      // A timer, not a callback on every display refresh: waking the browser 60 times a second
      // keeps its compositor busy even when nothing is drawn. The timer fires, one frame is drawn.
      const busy = performance.now() - lastActivity < SKY_ACTIVE_WINDOW_MS || currentComet;
      rafId = -1;
      waitTimer = setTimeout(() => {
        rafId = requestAnimationFrame(renderFrame);
      }, busy ? SKY_ACTIVE_FRAME_MS : SKY_IDLE_FRAME_MS);
    } else {
      rafId = null;
    }
  }

  function start() {
    if (rafId || paused || idle || isDestroyed) return;
    lastTime = performance.now();
    rafId = requestAnimationFrame(renderFrame);
  }

  function stop() {
    clearTimeout(waitTimer);
    if (rafId) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  }

  function onVisibilityChange() {
    if (document.hidden) {
      stop();
    } else if (mode === 'drifting' || (enableComets && !prefersReducedMotion.matches)) {
      start();
    }
  }

  document.addEventListener('visibilitychange', onVisibilityChange);

  // Drifting with nobody around: stop after ~30 s without pointer or key activity and resume on
  // the next one (AUDIT-34). The last frame stays drawn, so nothing flashes.
  const IDLE_MS = 30000;
  function onActivity() {
    lastActivity = performance.now();
    if (idle) {
      idle = false;
      if (mode === 'drifting' || (enableComets && !prefersReducedMotion.matches)) start();
    }
  }
  function onPointerMove(e) {
    mouse.x = e.clientX;
    mouse.y = e.clientY;
    mouse.inside = true;
    mouse.tx = (e.clientX / (window.innerWidth || width) - 0.5) * 2;
    mouse.ty = (e.clientY / (window.innerHeight || height) - 0.5) * 2;
    onActivity();
    // Waiting out a slow idle frame: draw now, the pointer just came back.
    if (rafId === -1 && mode === 'drifting') {
      clearTimeout(waitTimer);
      rafId = requestAnimationFrame(renderFrame);
    }
  }
  function onPointerLeave() { mouse.inside = false; }
  window.addEventListener('pointermove', onPointerMove, { passive: true });
  document.addEventListener('pointerleave', onPointerLeave);
  window.addEventListener('blur', onPointerLeave);
  window.addEventListener('pointerdown', onActivity, { passive: true });
  window.addEventListener('keydown', onActivity, { passive: true });
  const idleTimer = setInterval(() => {
    if (!idle && !paused && rafId !== null && performance.now() - lastActivity > IDLE_MS) {
      idle = true;
      stop();
    }
  }, 2000);
  window.__skyIdleMs = IDLE_MS;

  // Initial draw / start
  if (mode === 'still' && (!enableComets || prefersReducedMotion.matches)) {
    // Draw immediately without RAF
    ctx.clearRect(0, 0, width, height);
    if (mode !== 'off') {
      for (const st of stars) drawStar(st);
    }
    ctx.globalAlpha = 1.0;
  } else if (mode !== 'off') {
    start();
  }

  if (container) {
    container.append(canvas);
  }

  const handle = {
    canvas,
    isRunning: () => rafId !== null,
    /** For automated checks: what this sky is showing. */
    info: () => ({ stars: stars.length, mode, comets: enableComets, cursor: cursorOpt, drift: driftScale, cometScale, comet: !!currentComet }),
    /** Pause while nothing can see the sky (a web page covers it); resume when shown (AUDIT-34). */
    setPaused(p) {
      if (paused === p || isDestroyed) return;
      paused = p;
      if (p) stop();
      else if (mode === 'drifting' || (enableComets && !prefersReducedMotion.matches)) start();
    },
    destroy() {
      isDestroyed = true;
      stop();
      liveSkies.delete(handle);
      clearInterval(idleTimer);
      window.removeEventListener('pointermove', onPointerMove);
      document.removeEventListener('pointerleave', onPointerLeave);
      window.removeEventListener('blur', onPointerLeave);
      window.removeEventListener('pointerdown', onActivity);
      window.removeEventListener('keydown', onActivity);
      document.removeEventListener('visibilitychange', onVisibilityChange);
      canvas.remove();
    },
  };
  liveSkies.add(handle);
  return handle;
}

export function generateSky(type, width = 1280, height = 800) {
  const isFixture = document.body.classList.contains('fixture');
  const stars = [];
  if (type === 'private') {
    const ringRadii = [1.6, 2.0, 2.6];
    const ringOpacities = [0.25, 0.35, 0.5];
    const dotRadii = [0.6, 0.8, 1.0];
    const dotOpacities = [0.2, 0.3, 0.4];
    const totalCount = isFixture ? 120 : Math.round((width * height / (1280 * 800)) * 120);
    const ringCount = Math.round(totalCount * 0.4);
    for (let i = 0; i < totalCount; i++) {
      const x = Math.random() * width;
      const y = Math.random() * height;
      if (i < ringCount) {
        stars.push({
          x,
          y,
          r: ringRadii[Math.floor(Math.random() * ringRadii.length)],
          o: ringOpacities[Math.floor(Math.random() * ringOpacities.length)],
          ring: true,
          stroke: '#cdd9ee',
          w: 1.0,
        });
      } else {
        stars.push({
          x,
          y,
          r: dotRadii[Math.floor(Math.random() * dotRadii.length)],
          o: dotOpacities[Math.floor(Math.random() * dotOpacities.length)],
          fill: '#cdd9ee',
        });
      }
    }
    return stars;
  }

  const radii = [0.6, 0.7, 0.8, 0.9, 1.1, 1.3];
  const opacities = [0.25, 0.35, 0.45, 0.55, 0.7];
  let n = isFixture ? 170 : Math.round((width * height / (1280 * 800)) * 170);
  let maxX = width;
  if (type === 'expanded') {
    n = 46;
    maxX = 264;
  } else if (type === 'collapsed') {
    n = 18;
    maxX = 56;
  }

  for (let i = 0; i < n; i++) {
    stars.push({
      x: Math.random() * maxX,
      y: Math.random() * height,
      r: radii[Math.floor(Math.random() * radii.length)],
      o: opacities[Math.floor(Math.random() * opacities.length)],
      fill: '#ffffff',
    });
  }
  return stars;
}
