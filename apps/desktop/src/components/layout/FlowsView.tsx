import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  blankFlow,
  blankItem,
  clampItemCount,
  type FlowDocument,
  type FlowDraft,
  type FlowMenuCommand,
  moveItem,
} from "../../lib/flow";
import {
  createFlow,
  deleteFlow,
  exportFlowFile,
  importFlowFile,
  loadFlows,
  pickFlowToOpen,
  pickFlowToSave,
  updateFlow,
} from "../../native/flows";
import { selectableMeshModels, selectableModels } from "../../lib/modelChoice";
import type { ListedModel } from "../../native/models";
import { enqueueFlow } from "../../native/queue";
import { loadTemplates, type PromptTemplate } from "../../native/templates";
import { Button } from "../ui/button";
import { FlowItemCard } from "./FlowItemCard";
import { FlowPreview } from "./FlowPreview";

type FlowsViewProps = {
  onBack: () => void;
  command: FlowMenuCommand;
  library?: ListedModel[];
};

const fieldClass =
  "w-full rounded-md border border-zinc-300 bg-white px-2 py-1.5 text-sm text-zinc-900 dark:border-zinc-700 dark:bg-zinc-950 dark:text-zinc-100";

function draftOf(flow: FlowDocument): FlowDraft {
  return {
    name: flow.name,
    templateId: flow.templateId,
    path: flow.path,
    items: flow.items,
  };
}

function nextId(): string {
  return globalThis.crypto.randomUUID();
}

export function FlowsView({ onBack, command, library = [] }: FlowsViewProps) {
  const { t } = useTranslation();
  const imageModels = selectableModels(library, t("panel.unknownArchitecture"));
  const meshModels = selectableMeshModels(library);
  const [flows, setFlows] = useState<FlowDocument[]>([]);
  const [templates, setTemplates] = useState<PromptTemplate[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [queued, setQueued] = useState(false);

  const selected = flows.find((flow) => flow.id === selectedId) ?? null;
  const selectedRef = useRef(selected);
  selectedRef.current = selected;

  useEffect(() => {
    void loadFlows().then((rows) => {
      setFlows(rows);
      const first = rows[0];
      if (first !== undefined) {
        setSelectedId(first.id);
      }
    });
  }, []);

  useEffect(() => {
    void loadTemplates().then(setTemplates);
  }, []);

  function replace(flow: FlowDocument) {
    setFlows((current) => {
      const exists = current.some((entry) => entry.id === flow.id);
      return exists
        ? current.map((entry) => (entry.id === flow.id ? flow : entry))
        : [...current, flow];
    });
    setSelectedId(flow.id);
    setFailure(null);
    setQueued(false);
  }

  async function persist(next: FlowDocument) {
    replace(next);
    if (next.name.trim() === "") {
      return;
    }
    try {
      await updateFlow(next.id, draftOf(next));
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  async function create() {
    try {
      replace(await createFlow(blankFlow(t("flows.newName"), nextId())));
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  async function openFile() {
    try {
      const path = await pickFlowToOpen();
      if (path === null) {
        return;
      }
      replace(await importFlowFile(path));
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  async function saveFile() {
    if (selected === null) {
      return;
    }
    try {
      const path = selected.path ?? (await pickFlowToSave(selected.path));
      if (path === null) {
        return;
      }
      replace(await exportFlowFile(selected.id, path));
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  async function execute() {
    if (selected === null) {
      return;
    }
    if (selected.items.length === 0) {
      setQueued(false);
      setFailure(t("flows.needItems"));
      return;
    }
    try {
      await updateFlow(selected.id, draftOf(selected));
      await enqueueFlow(selected.id);
      setFailure(null);
      setQueued(true);
    } catch (error: unknown) {
      setQueued(false);
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  async function remove() {
    if (selectedId === null) {
      return;
    }
    try {
      await deleteFlow(selectedId);
      const remaining = flows.filter((flow) => flow.id !== selectedId);
      setFlows(remaining);
      setSelectedId(remaining[0]?.id ?? null);
      setFailure(null);
      setQueued(false);
    } catch (error: unknown) {
      setFailure(error instanceof Error ? error.message : String(error));
    }
  }

  const menuRef = useRef({ create, openFile, saveFile });
  menuRef.current = { create, openFile, saveFile };

  useEffect(() => {
    if (command.kind === "idle") {
      return;
    }
    const menu = menuRef.current;
    if (command.kind === "new") {
      void menu.create();
    } else if (command.kind === "open") {
      void menu.openFile();
    } else if (command.kind === "save") {
      void menu.saveFile();
    }
  }, [command]);

  return (
    <div className="flex min-h-0 flex-1 overflow-hidden">
      <aside className="flex w-64 shrink-0 flex-col gap-3 border-r border-zinc-200 p-3 dark:border-zinc-800">
        <Button variant="ghost" className="justify-start" onClick={onBack}>
          {t("settings.back")}
        </Button>
        <Button onClick={() => void create()}>{t("flows.create")}</Button>
        <ul className="flex min-h-0 flex-1 flex-col gap-1 overflow-y-auto">
          {flows.map((flow) => (
            <li key={flow.id}>
              <button
                type="button"
                aria-pressed={flow.id === selectedId}
                className="w-full rounded-md px-2 py-1.5 text-left text-sm hover:bg-zinc-200 aria-pressed:bg-zinc-200 dark:hover:bg-zinc-800 dark:aria-pressed:bg-zinc-800"
                onClick={() => setSelectedId(flow.id)}
              >
                {flow.name}
              </button>
            </li>
          ))}
        </ul>
        {flows.length === 0 ? <p className="text-sm text-zinc-500">{t("flows.empty")}</p> : null}
      </aside>
      <section className="flex min-h-0 min-w-0 flex-[1.2] flex-col gap-4 overflow-y-auto p-6">
        <h1 className="text-lg font-semibold">{t("flows.title")}</h1>
        {failure !== null ? <p role="alert">{failure}</p> : null}
        {queued ? <p>{t("flows.queued")}</p> : null}
        {selected === null ? null : (
          <div className="flex max-w-2xl flex-col gap-4">
            <label className="flex flex-col gap-1 text-sm font-medium">
              {t("flows.name")}
              <input
                aria-label={t("flows.name")}
                className={fieldClass}
                value={selected.name}
                onChange={(event) => void persist({ ...selected, name: event.target.value })}
              />
            </label>
            {selected.path !== null ? (
              <p className="text-sm text-zinc-500">{t("flows.savedTo", { path: selected.path })}</p>
            ) : null}
            <div className="flex flex-wrap gap-2">
              <Button onClick={() => void saveFile()}>{t("flows.saveFile")}</Button>
              <Button onClick={() => void execute()}>{t("flows.execute")}</Button>
              <Button variant="ghost" onClick={() => void remove()}>
                {t("templates.delete")}
              </Button>
            </div>
            <ol className="flex flex-col gap-3">
              {selected.items.map((item, index) => (
                <li key={item.id}>
                  <FlowItemCard
                    item={item}
                    index={index}
                    total={selected.items.length}
                    templates={templates}
                    imageModels={imageModels}
                    meshModels={meshModels}
                    onChange={(nextItem) => {
                      const current = selectedRef.current;
                      if (current === null) {
                        return;
                      }
                      const items = current.items.slice();
                      items[index] = {
                        ...nextItem,
                        count: clampItemCount(nextItem.count),
                      };
                      void persist({ ...current, items });
                    }}
                    onMove={(from, to) => {
                      const current = selectedRef.current;
                      if (current === null) {
                        return;
                      }
                      void persist({ ...current, items: moveItem(current.items, from, to) });
                    }}
                    onRemove={() => {
                      const current = selectedRef.current;
                      if (current === null) {
                        return;
                      }
                      void persist({
                        ...current,
                        items: current.items.filter((entry) => entry.id !== item.id),
                      });
                    }}
                  />
                </li>
              ))}
            </ol>
            <Button
              onClick={() =>
                void persist({
                  ...selected,
                  items: [...selected.items, blankItem(nextId())],
                })
              }
            >
              {t("flows.addItem")}
            </Button>
          </div>
        )}
      </section>
      <FlowPreview queued={queued} />
    </div>
  );
}
