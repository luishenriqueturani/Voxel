type TitledPrompt = {
  prompt: string;
  title?: string;
};

export function promptLabel(entry: TitledPrompt): string {
  const title = (entry.title ?? "").trim();
  if (title === "" || isJunkTitle(title) || echoesPrompt(title, entry.prompt)) {
    return entry.prompt;
  }
  return title;
}

export function isJunkTitle(title: string): boolean {
  const lower = title.trim().toLowerCase();
  return (
    lower === "thinking" ||
    lower.startsWith("thinking") ||
    lower === "pensando" ||
    lower.startsWith("pensando")
  );
}

function echoesPrompt(title: string, prompt: string): boolean {
  const short = title.trim().toLowerCase();
  const full = prompt.trim().toLowerCase();
  if (short.length < 16) {
    return false;
  }
  return full.startsWith(short) || short.startsWith(full);
}
