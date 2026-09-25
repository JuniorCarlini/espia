// Boot splash animation: the dot-matrix wordmark lights up dot-by-dot, in a
// random order picked fresh every launch, then blinks three times together
// before the splash fades out. See docs/adr — the animation itself has no
// per-dot growth (opacity only), matching the variant chosen after trying a
// few in a throwaway prototype.

const ASSEMBLE_SPREAD_S = 1.6;
const BLINK_ITERATIONS = 3;
const BLINK_DURATION_S = 0.45;
const SETTLE_PAUSE_S = 0.45;

function shuffle(items) {
  const order = [...items];
  for (let i = order.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [order[i], order[j]] = [order[j], order[i]];
  }
  return order;
}

// Starts the animation and resolves once it has fully played out (assembly +
// three blinks), regardless of how long the real data takes to load.
export function playBootAnimation() {
  const svg = document.querySelector("#boot-splash .boot-logo");
  const circles = svg ? Array.from(svg.querySelectorAll("circle")) : [];
  if (!svg || circles.length === 0) return Promise.resolve();

  let maxDelay = 0;
  shuffle(circles).forEach((circle, index) => {
    const delay = (index * ASSEMBLE_SPREAD_S) / circles.length + Math.random() * 0.03;
    circle.style.setProperty("--rd", `${delay.toFixed(3)}s`);
    maxDelay = Math.max(maxDelay, delay);
  });

  const blinkDelay = maxDelay + SETTLE_PAUSE_S;
  circles.forEach((circle) => circle.style.setProperty("--rblink", `${blinkDelay.toFixed(3)}s`));

  svg.classList.add("is-playing");

  const totalMs = (blinkDelay + BLINK_ITERATIONS * BLINK_DURATION_S) * 1000;
  return new Promise((resolve) => setTimeout(resolve, totalMs));
}

// Fades the splash out and removes it once the transition ends.
export function hideBootSplash() {
  const splash = document.getElementById("boot-splash");
  if (!splash) return;
  splash.addEventListener("transitionend", () => splash.remove(), { once: true });
  splash.classList.add("is-hidden");
}
