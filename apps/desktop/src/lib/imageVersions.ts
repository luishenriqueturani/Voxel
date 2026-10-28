import type { GalleryImage, ImageVersion } from "../native/generation";

export function versionsOf(slide: GalleryImage): ImageVersion[] {
  if (slide.versions && slide.versions.length > 0) {
    return slide.versions;
  }
  return [
    {
      src: slide.src,
      path: slide.path,
      seed: slide.seed,
      transparent: slide.transparent,
    },
  ];
}

export function showVersion(slide: GalleryImage, index: number): GalleryImage {
  const versions = versionsOf(slide);
  const chosen = versions[index];
  if (chosen === undefined) {
    return slide;
  }
  return {
    ...slide,
    src: chosen.src,
    path: chosen.path,
    seed: chosen.seed,
    transparent: chosen.transparent,
    versions,
    versionIndex: index,
  };
}

export function applyEditResult(
  slides: readonly GalleryImage[],
  sourceIndex: number,
  results: readonly GalleryImage[],
): { slides: GalleryImage[]; index: number } {
  if (results.length === 0) {
    return { slides: slides.map((slide) => ({ ...slide })), index: sourceIndex };
  }
  const source = slides[sourceIndex];
  if (source === undefined) {
    const added = results.map((image) => ({ ...image }));
    return { slides: [...slides, ...added], index: slides.length + added.length - 1 };
  }
  const versions = [
    ...versionsOf(source),
    ...results.map((image) => ({
      src: image.src,
      path: image.path,
      seed: image.seed,
      transparent: image.transparent,
    })),
  ];
  const next = slides.map((slide) => ({ ...slide }));
  next[sourceIndex] = showVersion({ ...source, versions }, versions.length - 1);
  return { slides: next, index: sourceIndex };
}
