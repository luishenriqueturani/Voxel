export function pushSlides<T extends object>(
  history: readonly (readonly T[])[],
  slides: readonly T[],
): T[][] {
  if (slides.length === 0) {
    return history.map((entry) => [...entry]);
  }
  const next = history.map((entry) => [...entry]);
  next.push(slides.map((slide) => ({ ...slide })));
  return next.length > 20 ? next.slice(next.length - 20) : next;
}

export function popSlides<T>(history: readonly (readonly T[])[]): {
  history: T[][];
  slides: T[] | null;
} {
  if (history.length === 0) {
    return { history: [], slides: null };
  }
  const slides = history[history.length - 1];
  return {
    history: history.slice(0, -1).map((entry) => [...entry]),
    slides: slides === undefined ? null : [...slides],
  };
}
