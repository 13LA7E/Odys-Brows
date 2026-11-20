// Sky rendering on canvas (DESIGN.md §5, PHASES.md P2-01).
// Still mode renders once at device pixel ratio (0% idle CPU).
// Drifting and Shooting Stars (comets) run slow, subtle animations when enabled,
// paused immediately whenever the window or tab is hidden or when reduced motion is preferred.

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
  if (!options.stars && mode !== 'off') {
    stars = generateSky(options.type || (isPrivate ? 'private' : 'newtab'), width, height);
  } else if (!isFixture && mode !== 'off' && (width > 1280 || height > 800)) {
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

  // Pre-calculate deterministic drift velocities (very slow: 0.02 - 0.06 px/frame)
  for (let i = 0; i < stars.length; i++) {
    const angle = ((i * 47) % 360) * (Math.PI / 180);
    const speed = 0.02 + ((i * 13) % 4) * 0.01;
    stars[i].vx = Math.cos(angle) * speed;
    stars[i].vy = Math.sin(angle) * speed;
  }

  let rafId = null;
  let lastTime = 0;
  let nextCometTime = performance.now() + 15000 + Math.random() * 15000;
  let currentComet = null;
  let isDestroyed = false;

  const prefersReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

  function drawStar(st) {
    ctx.beginPath();
    ctx.arc(st.x, st.y, st.r, 0, Math.PI * 2);
    if (st.ring) {
      ctx.strokeStyle = st.stroke || '#cdd9ee';
      ctx.lineWidth = st.w || 1.0;
      ctx.globalAlpha = st.o;
      ctx.stroke();
    } else {
      ctx.fillStyle = st.fill || (isPrivate ? '#cdd9ee' : '#ffffff');
      ctx.globalAlpha = st.o;
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
    if (document.hidden) {
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

    for (const st of stars) {
      if (mode === 'drifting' && canAnimate) {
        st.x += st.vx * (dt / 0.016);
        st.y += st.vy * (dt / 0.016);
        if (st.x < 0) st.x += width;
        if (st.x > width) st.x -= width;
        if (st.y < 0) st.y += height;
        if (st.y > height) st.y -= height;
      }
      drawStar(st);
    }

    // Shooting stars (rare, subtle comets across new tab)
    if (enableComets && canAnimate) {
      if (!currentComet && now >= nextCometTime) {
        // Spawn a new comet
        const startX = 200 + Math.random() * (width - 400);
        const startY = 50 + Math.random() * 200;
        const angle = (25 + Math.random() * 20) * (Math.PI / 180); // gentle downward slant
        currentComet = {
          x: startX,
          y: startY,
          angle,
          length: 50,
          speed: 380, // px per second
          distance: 0,
          maxDistance: 280,
          alpha: 0,
        };
      }

      if (currentComet) {
        currentComet.distance += currentComet.speed * dt;
        currentComet.x += Math.cos(currentComet.angle) * currentComet.speed * dt;
        currentComet.y += Math.sin(currentComet.angle) * currentComet.speed * dt;

        // Fade in then fade out
        const frac = currentComet.distance / currentComet.maxDistance;
        if (frac < 0.2) {
          currentComet.alpha = (frac / 0.2) * 0.45;
        } else if (frac > 0.7) {
          currentComet.alpha = (1 - (frac - 0.7) / 0.3) * 0.45;
        } else {
          currentComet.alpha = 0.45;
        }

        drawComet(currentComet);

        if (frac >= 1) {
          currentComet = null;
          nextCometTime = now + 15000 + Math.random() * 15000;
        }
      }
    }

    ctx.globalAlpha = 1.0;

    if (mode === 'drifting' || (enableComets && canAnimate)) {
      rafId = requestAnimationFrame(renderFrame);
    } else {
      rafId = null;
    }
  }

  function start() {
    if (rafId) return;
    lastTime = performance.now();
    rafId = requestAnimationFrame(renderFrame);
  }

  function stop() {
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

  return {
    canvas,
    setMode(newMode) {
      if (mode === newMode) return;
      // Stop old loop if any
      stop();
      return createSky(container, { ...options, mode: newMode, stars });
    },
    destroy() {
      isDestroyed = true;
      stop();
      document.removeEventListener('visibilitychange', onVisibilityChange);
      canvas.remove();
    },
  };
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
