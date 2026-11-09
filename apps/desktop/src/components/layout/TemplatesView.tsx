import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { composeGeneration } from "../../lib/templatePrompt";
import {
  createTemplate,
  deleteTemplate,
  duplicateTemplate,
  loadTemplates,
  type PromptTemplate,
  renameTemplate,
  type TemplateDraft,
  updateTemplate,
} from "../../native/templates";
import { Button } from "../ui/button";
import { TemplatesGuide } from "./TemplatesGuide";

type TemplatesViewProps = {
  onBack: () => void;
};

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

function draftOf(template: PromptTemplate): TemplateDraft {
  const { id: _id, ...draft } = template;
  return draft;
}

function blankDraft(): TemplateDraft {
  return {
    name: "",
    body: "",
    negative: "",
    placement: "after",
  };
}

export function TemplatesView({ onBack }: TemplatesViewProps) {
  const { t } = useTranslation();
  const [templates, setTemplates] = useState<PromptTemplate[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draft, setDraft] = useState<TemplateDraft | null>(null);
  const [sample, setSample] = useState("");
  const [failure, setFailure] = useState<string | null>(null);

  useEffect(() => {
    void loadTemplates().then((rows) => {
      setTemplates(rows);
      const first = rows[0];
      if (first !== undefined) {
        setSelectedId(first.id);
        setDraft(draftOf(first));
      }
    });
  }, []);

  function select(template: PromptTemplate) {
    setSelectedId(template.id);
    setDraft(draftOf(template));
    setFailure(null);
  }

  function replace(template: PromptTemplate) {
    setTemplates((current) => current.map((item) => (item.id === template.id ? template : item)));
    setSelectedId(template.id);
    setDraft(draftOf(template));
    setFailure(null);
  }

  function create() {
    setSelectedId(null);
    setDraft(blankDraft());
    setSample("");
    setFailure(null);
  }

  async function duplicate() {
    if (selectedId === null || draft === null) {
      return;
    }
    try {
      const copy = await duplicateTemplate(
        selectedId,
        t("templates.copyName", { name: draft.name }),
      );
      setTemplates((current) => [...current, copy]);
      select(copy);
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  async function rename() {
    if (selectedId === null || draft === null) {
      return;
    }
    try {
      const renamed = await renameTemplate(selectedId, draft.name);
      setTemplates((current) =>
        current.map((item) => (item.id === renamed.id ? { ...item, name: renamed.name } : item)),
      );
      setDraft((current) => (current === null ? current : { ...current, name: renamed.name }));
      setFailure(null);
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  async function save() {
    if (draft === null) {
      return;
    }
    try {
      if (selectedId === null) {
        const created = await createTemplate(draft);
        setTemplates((current) => [...current, created]);
        select(created);
        return;
      }
      replace(await updateTemplate(selectedId, draft));
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  async function remove() {
    if (selectedId === null) {
      return;
    }
    try {
      await deleteTemplate(selectedId);
      const remaining = templates.filter((item) => item.id !== selectedId);
      setTemplates(remaining);
      const next = remaining[0];
      if (next === undefined) {
        setSelectedId(null);
        setDraft(null);
      } else {
        select(next);
      }
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  const composed = draft === null ? null : composeGeneration(draft, sample);
  const nameReady = draft !== null && draft.name.trim() !== "";

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <aside className="flex w-64 shrink-0 flex-col gap-3 border-r border-zinc-200 p-3 dark:border-zinc-800">
        <Button variant="ghost" className="justify-start" onClick={onBack}>
          {t("settings.back")}
        </Button>
        <Button onClick={create}>{t("templates.create")}</Button>
        <ul className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto">
          {templates.map((template) => (
            <li key={template.id}>
              <button
                type="button"
                aria-pressed={template.id === selectedId}
                className="w-full rounded-md px-2 py-1.5 text-left text-sm hover:bg-zinc-200 aria-pressed:bg-zinc-200 dark:hover:bg-zinc-800 dark:aria-pressed:bg-zinc-800"
                onClick={() => select(template)}
              >
                {template.name}
              </button>
            </li>
          ))}
        </ul>
        {templates.length === 0 ? (
          <p className="text-sm text-zinc-500">{t("templates.empty")}</p>
        ) : null}
      </aside>
      <section className="flex min-h-0 min-w-0 flex-1">
        <div className="flex min-h-0 min-w-0 flex-1 flex-col gap-4 overflow-y-auto p-6">
          <h1 className="text-lg font-semibold">{t("templates.title")}</h1>
          {failure !== null ? <p role="alert">{failure}</p> : null}
          {draft === null || composed === null ? (
            <p className="text-sm text-zinc-500">{t("templates.pickOrCreate")}</p>
          ) : (
            <form
              className="flex max-w-xl flex-col gap-3"
              onSubmit={(event) => {
                event.preventDefault();
                void save();
              }}
            >
              <label className="flex flex-col gap-1 text-sm font-medium">
                {t("templates.name")}
                <input
                  aria-label={t("templates.name")}
                  className={fieldClass}
                  placeholder={t("templates.newName")}
                  value={draft.name}
                  onChange={(event) => setDraft({ ...draft, name: event.target.value })}
                />
              </label>
              <div className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  variant="ghost"
                  disabled={!nameReady || selectedId === null}
                  onClick={() => void rename()}
                >
                  {t("templates.rename")}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  disabled={selectedId === null}
                  onClick={() => void duplicate()}
                >
                  {t("templates.duplicate")}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  disabled={selectedId === null}
                  onClick={() => void remove()}
                >
                  {t("templates.delete")}
                </Button>
              </div>
              <label className="flex flex-col gap-1 text-sm font-medium">
                {t("templates.body")}
                <textarea
                  aria-label={t("templates.body")}
                  className={`${fieldClass} min-h-24`}
                  value={draft.body}
                  onChange={(event) => setDraft({ ...draft, body: event.target.value })}
                />
              </label>
              <p className="text-xs text-zinc-500">{t("templates.bodyHint")}</p>
              <label className="flex items-center gap-2 text-sm font-medium">
                <input
                  type="checkbox"
                  checked={draft.placement === "before"}
                  onChange={(event) =>
                    setDraft({ ...draft, placement: event.target.checked ? "before" : "after" })
                  }
                />
                {t("templates.placeBefore")}
              </label>
              <p className="text-xs text-zinc-500">{t("templates.placeHint")}</p>
              <label className="flex flex-col gap-1 text-sm font-medium">
                {t("templates.negative")}
                <textarea
                  aria-label={t("templates.negative")}
                  className={`${fieldClass} min-h-16`}
                  value={draft.negative}
                  onChange={(event) => setDraft({ ...draft, negative: event.target.value })}
                />
              </label>
              <Button type="submit" disabled={!nameReady}>
                {t("templates.save")}
              </Button>
              <label className="flex flex-col gap-1 text-sm font-medium">
                {t("templates.sample")}
                <input
                  aria-label={t("templates.sample")}
                  className={fieldClass}
                  value={sample}
                  onChange={(event) => setSample(event.target.value)}
                />
              </label>
              <div>
                <h2 className="text-sm font-medium">{t("templates.final")}</h2>
                <p>{composed.prompt}</p>
                <p>
                  {t("templates.negative")}: {composed.negative || t("templates.noNegative")}
                </p>
              </div>
            </form>
          )}
        </div>
        <TemplatesGuide />
      </section>
    </div>
  );
}
