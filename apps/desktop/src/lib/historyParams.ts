import {
  ASPECTS,
  type AspectRatio,
  frameSize,
  type GenerationParams,
} from "../components/layout/PromptPanel";

const RESOLUTIONS = [512, 768, 1024] as const;

type HistoryShape = {
  width: number;
  height: number;
  steps: number;
  cfg: number;
  seed: number;
};

export function paramsFromHistory(entry: HistoryShape, count: number): GenerationParams {
  const longSide = Math.max(entry.width, entry.height);
  const resolution = RESOLUTIONS.reduce((best, candidate) =>
    Math.abs(candidate - longSide) < Math.abs(best - longSide) ? candidate : best,
  );
  return {
    resolution,
    aspect: aspectFor(entry.width, entry.height, resolution),
    width: entry.width,
    height: entry.height,
    steps: entry.steps,
    cfg: entry.cfg,
    seed: String(entry.seed),
    count,
  };
}

function aspectFor(width: number, height: number, resolution: number): AspectRatio {
  const target = height === 0 ? 1 : width / height;
  let best: AspectRatio = "1:1";
  let bestScore = Number.POSITIVE_INFINITY;
  for (const aspect of ASPECTS) {
    const frame = frameSize(resolution, aspect);
    if (frame.width !== width || frame.height !== height) {
      continue;
    }
    const [widthText, heightText] = aspect.split(":");
    const ratioWidth = Number(widthText);
    const ratioHeight = Number(heightText);
    if (!Number.isFinite(ratioWidth) || !Number.isFinite(ratioHeight) || ratioHeight === 0) {
      continue;
    }
    const score = Math.abs(ratioWidth / ratioHeight - target);
    if (score < bestScore) {
      best = aspect;
      bestScore = score;
    }
  }
  return best;
}
