export type MeshOrigin = "text" | "images";

export type MeshView = {
  path: string;
  src: string;
  seed: number | null;
};

export type MeshPick = {
  views: MeshView[];
  primaryPath: string;
};

export function viewName(path: string): string {
  const parts = path.split(/[/\\]/);
  return parts[parts.length - 1] || path;
}

export function meshViewsOf(
  slides: readonly { path: string; src: string; seed: number | null }[],
): MeshView[] {
  return slides.flatMap((slide) =>
    slide.path === "" ? [] : [{ path: slide.path, src: slide.src, seed: slide.seed }],
  );
}

export function moveView<T>(items: readonly T[], index: number, direction: -1 | 1): T[] {
  const current = items[index];
  const other = items[index + direction];
  if (current === undefined || other === undefined) {
    return items.slice();
  }
  const next = items.slice();
  next[index] = other;
  next[index + direction] = current;
  return next;
}

export function pickForPath(views: readonly MeshView[], path: string): MeshPick {
  if (views.some((view) => view.path === path)) {
    return { views: [...views], primaryPath: path };
  }
  return { views: [{ path, src: "", seed: null }], primaryPath: path };
}

export function viewSrc(src: string): string | null {
  if (
    src.startsWith("data:") ||
    src.startsWith("blob:") ||
    src.startsWith("http://") ||
    src.startsWith("https://")
  ) {
    return src;
  }
  return null;
}
