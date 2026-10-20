import { Star } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { paramsFromHistory } from "../../lib/historyParams";
import { loadRecentPrompts, type PromptEntry, setPromptFavorite } from "../../native/history";
import type { GenerationParams } from "./PromptPanel";

type RecentPromptsProps = {
  count: number;
  onPromptChange: (value: string) => void;
  onNegativePromptChange: (value: string) => void;
  onModelChange: (id: string) => void;
  onParamsChange: (params: GenerationParams) => void;
  onError: (error: unknown) => void;
  onRestore: (entry: PromptEntry) => void;
};

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

export function RecentPrompts({
  count,
  onPromptChange,
  onNegativePromptChange,
  onModelChange,
  onParamsChange,
  onError,
  onRestore,
}: RecentPromptsProps) {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<PromptEntry[]>([]);
  const [query, setQuery] = useState("");

  useEffect(() => {
    let cancelled = false;
    void loadRecentPrompts()
      .then((loaded) => {
        if (!cancelled) {
          setEntries(loaded);
        }
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          onError(error);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [onError]);

  async function toggle(entry: PromptEntry) {
    const pinned = !entry.pinned;
    try {
      await setPromptFavorite(entry, pinned);
      const loaded = await loadRecentPrompts();
      setEntries(loaded);
    } catch (error) {
      onError(error);
    }
  }

  function apply(entry: PromptEntry) {
    onPromptChange(entry.prompt);
    onNegativePromptChange(entry.negativePrompt);
    onModelChange(entry.modelId);
    onParamsChange(paramsFromHistory(entry, count));
    onRestore(entry);
  }

  const needle = query.trim().toLowerCase();
  const visible = entries.filter((entry) => {
    if (needle === "") {
      return true;
    }
    return (
      entry.prompt.toLowerCase().includes(needle) ||
      entry.negativePrompt.toLowerCase().includes(needle)
    );
  });

  return (
    <section className="flex min-h-0 flex-1 flex-col gap-2 border-t border-zinc-200 pt-3 dark:border-zinc-800">
      <h2 className="text-sm font-medium">{t("panel.recent")}</h2>
      <input
        type="search"
        aria-label={t("panel.recentSearch")}
        placeholder={t("panel.recentSearch")}
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        className={fieldClass}
      />
      {visible.length === 0 ? (
        <p className="text-sm text-zinc-500">
          {needle === "" ? t("panel.recentEmpty") : t("panel.recentNone")}
        </p>
      ) : (
        <ul className="flex flex-col gap-1">
          {visible.map((entry) => (
            <li key={entryKey(entry)} className="flex items-start gap-1">
              <button
                type="button"
                title={entry.prompt}
                className="min-w-0 flex-1 truncate text-left text-sm"
                onClick={() => apply(entry)}
              >
                {entry.prompt}
              </button>
              <button
                type="button"
                aria-pressed={entry.pinned}
                aria-label={entry.pinned ? t("panel.unpin") : t("panel.pin")}
                className="shrink-0 rounded-md p-1 text-zinc-500 hover:text-zinc-900 dark:hover:text-zinc-100"
                onClick={() => {
                  void toggle(entry);
                }}
              >
                <Star aria-hidden="true" size={14} fill={entry.pinned ? "currentColor" : "none"} />
              </button>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function entryKey(entry: PromptEntry): string {
  return [
    entry.pinned ? "1" : "0",
    entry.prompt,
    entry.negativePrompt,
    entry.modelId,
    entry.steps,
    entry.width,
    entry.height,
    entry.cfg,
    entry.seed,
  ].join("|");
}
