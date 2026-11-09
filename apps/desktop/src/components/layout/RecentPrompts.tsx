import { Star } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { paramsFromHistory } from "../../lib/historyParams";
import { promptLabel } from "../../lib/promptTitle";
import {
  hasOlderPrompts,
  listenToPromptTitle,
  loadOlderPrompts,
  loadRecentPrompts,
  type PromptEntry,
  setPromptFavorite,
} from "../../native/history";
import type { GenerationParams } from "./PromptPanel";
import { RecentPromptDialog } from "./RecentPromptDialog";

type RecentPromptsProps = {
  pending?: string | null;
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
  pending = null,
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
  const [hasMore, setHasMore] = useState(false);
  const [selected, setSelected] = useState<PromptEntry | null>(null);
  const expanded = useRef(false);

  const load = useCallback(async (showAll: boolean) => {
    const page = await loadRecentPrompts();
    if (!showAll) {
      setEntries(page);
      setHasMore(await hasOlderPrompts());
      return;
    }
    setEntries(mergeEntries(page, await loadOlderPrompts()));
    setHasMore(false);
  }, []);

  useEffect(() => {
    let cancelled = false;
    void load(expanded.current).catch((error: unknown) => {
      if (!cancelled) {
        onError(error);
      }
    });
    return () => {
      cancelled = true;
    };
  }, [load, onError]);

  useEffect(() => {
    let cancelled = false;
    let stop = () => {};
    void listenToPromptTitle((payload) => {
      if (cancelled) {
        return;
      }
      void load(expanded.current).catch((error: unknown) => {
        if (!cancelled) {
          onError(error);
        }
      });
      setSelected((current) =>
        current !== null && current.prompt === payload.prompt
          ? { ...current, title: payload.title }
          : current,
      );
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
        return;
      }
      stop = unlisten;
    });
    return () => {
      cancelled = true;
      stop();
    };
  }, [load, onError]);

  async function toggle(entry: PromptEntry) {
    const pinned = !entry.pinned;
    try {
      await setPromptFavorite(entry, pinned);
      await load(expanded.current);
    } catch (error) {
      onError(error);
    }
  }

  async function showRest() {
    expanded.current = true;
    try {
      await load(true);
    } catch (error) {
      expanded.current = false;
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
      entry.negativePrompt.toLowerCase().includes(needle) ||
      promptLabel(entry).toLowerCase().includes(needle)
    );
  });
  const running =
    pending !== null &&
    pending.trim() !== "" &&
    needle === "" &&
    !entries.some((entry) => entry.prompt === pending)
      ? pending
      : null;

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
      <div className="min-h-0 flex-1 overflow-y-auto">
        {visible.length === 0 && running === null ? (
          <p className="text-sm text-zinc-500">
            {needle === "" ? t("panel.recentEmpty") : t("panel.recentNone")}
          </p>
        ) : (
          <ul className="flex flex-col gap-1">
            {running === null ? null : (
              <li className="flex items-start gap-1 text-sm">
                <span title={running} className="min-w-0 flex-1 truncate">
                  {running}
                </span>
                <span className="shrink-0 text-zinc-500">{t("panel.recentRunning")}</span>
              </li>
            )}
            {visible.map((entry) => (
              <li key={entryKey(entry)} className="flex items-start gap-1">
                <button
                  type="button"
                  title={entry.prompt}
                  className="min-w-0 flex-1 truncate text-left text-sm"
                  onClick={() => setSelected(entry)}
                >
                  {promptLabel(entry)}
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
                  <Star
                    aria-hidden="true"
                    size={14}
                    fill={entry.pinned ? "currentColor" : "none"}
                  />
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
      {hasMore ? (
        <button
          type="button"
          className="shrink-0 text-left text-sm underline-offset-2 hover:underline"
          onClick={() => void showRest()}
        >
          {t("panel.recentMore")}
        </button>
      ) : null}
      {selected ? (
        <RecentPromptDialog
          entry={selected}
          onClose={() => setSelected(null)}
          onRepeat={() => {
            apply(selected);
            setSelected(null);
          }}
          onError={onError}
        />
      ) : null}
    </section>
  );
}

function mergeEntries(
  current: readonly PromptEntry[],
  extra: readonly PromptEntry[],
): PromptEntry[] {
  const seen = new Set(current.map(entryKey));
  return [...current, ...extra.filter((entry) => !seen.has(entryKey(entry)))];
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
