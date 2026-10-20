const IMAGE = /\.(png|jpe?g|webp|bmp)$/i;

export function firstImagePath(paths: readonly string[]): string | null {
  return paths.find((path) => IMAGE.test(path.trim())) ?? null;
}
