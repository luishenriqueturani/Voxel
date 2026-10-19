import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ModelSuggestion, ProfileCatalog, SetupChoices, SetupStep } from "../../native/setup";
import { Button } from "../ui/button";

const EMPTY_MODELS: ModelSuggestion[] = [];

const BACKENDS = ["vulkan", "rocm", "cuda", "cpu"] as const;
const PROFILES = ["minimo", "basico", "intermediario", "avancado", "alto"] as const;

type WelcomeViewProps = {
  steps: SetupStep[];
  catalog: ProfileCatalog[];
  backend: string | null;
  confirming: boolean;
  image: string | null;
  outcome: string | null;
  detail: string;
  onConfirm: (choices: SetupChoices) => void;
  onContinue: () => void;
};

function formatBytes(bytes: number, language: string): string {
  const gib = bytes / 1024 ** 3;
  const value = gib >= 1 ? gib : bytes / 1024 ** 2;
  const unit = gib >= 1 ? "GiB" : "MiB";
  return `${value.toLocaleString(language, { maximumFractionDigits: 1 })} ${unit}`;
}

export function WelcomeView({
  steps,
  catalog,
  backend,
  confirming,
  image,
  outcome,
  detail,
  onConfirm,
  onContinue,
}: WelcomeViewProps) {
  const { t, i18n } = useTranslation();
  const detected = steps.find((step) => step.id === "profile" && step.state === "done")?.detail;
  const finished = detected !== undefined;
  const [advanced, setAdvanced] = useState(false);
  const [profileOverride, setProfileOverride] = useState<string | null>(null);
  const [backendOverride, setBackendOverride] = useState<string | null>(null);
  const profile = profileOverride ?? detected ?? "minimo";
  const chosenBackend = backendOverride ?? backend ?? "cpu";
  const models = catalog.find((entry) => entry.profile === profile)?.models ?? EMPTY_MODELS;
  const [selected, setSelected] = useState<string[]>([]);

  useEffect(() => {
    const next = catalog.find((entry) => entry.profile === profile)?.models ?? EMPTY_MODELS;
    setSelected(next.filter((model) => !model.optional).map((model) => model.id));
  }, [catalog, profile]);

  const labelFor = (step: SetupStep) => {
    if (step.id === "hardware") {
      return t("setup.hardware", { detail: step.detail });
    }
    if (step.id === "profile") {
      return t("setup.profile", { profile: t(`setup.profiles.${step.detail}`) });
    }
    return t("setup.backend", { backend: t(`setup.backends.${step.id}`) });
  };

  function toggleModel(id: string) {
    setSelected((current) =>
      current.includes(id) ? current.filter((item) => item !== id) : [...current, id],
    );
  }

  return (
    <main className="flex min-h-0 flex-1 flex-col gap-6 overflow-auto p-8">
      <div className="max-w-xl">
        <h1 id="setup-title" className="text-2xl font-semibold">
          {t("setup.title")}
        </h1>
        <p className="mt-2 text-sm text-zinc-600 dark:text-zinc-400">{t("setup.lead")}</p>
      </div>
      <ol aria-label={t("setup.progress")} className="flex max-w-xl flex-col gap-2">
        {steps.map((step) => (
          <li
            key={step.id}
            className="flex items-baseline justify-between gap-4 rounded-md border border-zinc-200 px-3 py-2 text-sm dark:border-zinc-800"
          >
            <span>{labelFor(step)}</span>
            <span className="shrink-0 text-zinc-500">{t(`setup.state.${step.state}`)}</span>
          </li>
        ))}
      </ol>
      {finished ? (
        <section className="flex max-w-xl flex-col gap-3" aria-label={t("setup.models")}>
          <h2 className="text-sm font-medium">{t("setup.models")}</h2>
          <p className="text-sm text-zinc-600 dark:text-zinc-400">{t("setup.modelsLead")}</p>
          <ul className="flex flex-col gap-2">
            {models.map((model) => (
              <li key={model.id}>
                <label className="flex items-start gap-3 rounded-md border border-zinc-200 px-3 py-2 text-sm dark:border-zinc-800">
                  <input
                    type="checkbox"
                    className="mt-1"
                    checked={selected.includes(model.id)}
                    onChange={() => toggleModel(model.id)}
                  />
                  <span>
                    <span className="font-medium">{t(`setup.model.${model.id}`)}</span>
                    {model.optional ? (
                      <span className="ml-2 text-zinc-500">{t("setup.optional")}</span>
                    ) : null}
                    <span className="mt-1 block text-zinc-600 dark:text-zinc-400">
                      {formatBytes(model.bytes, i18n.language)} · {model.license}
                    </span>
                  </span>
                </label>
              </li>
            ))}
          </ul>
          <Button variant="ghost" onClick={() => setAdvanced((current) => !current)}>
            {t("setup.advanced")}
          </Button>
          {advanced ? (
            <div className="flex flex-col gap-3">
              <label className="flex flex-col gap-1 text-sm">
                {t("setup.profileLabel")}
                <select
                  className="rounded-md border border-zinc-300 bg-transparent px-2 py-1 dark:border-zinc-700"
                  value={profile}
                  onChange={(event) => setProfileOverride(event.target.value)}
                >
                  {PROFILES.map((id) => (
                    <option key={id} value={id}>
                      {t(`setup.profiles.${id}`)}
                    </option>
                  ))}
                </select>
              </label>
              <label className="flex flex-col gap-1 text-sm">
                {t("setup.backendLabel")}
                <select
                  className="rounded-md border border-zinc-300 bg-transparent px-2 py-1 dark:border-zinc-700"
                  value={chosenBackend}
                  onChange={(event) => setBackendOverride(event.target.value)}
                >
                  {BACKENDS.map((id) => (
                    <option key={id} value={id}>
                      {t(`setup.backends.${id}`)}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          ) : null}
          {image === null ? (
            <Button
              disabled={confirming}
              onClick={() =>
                onConfirm({
                  models: selected,
                  profile,
                  backend: chosenBackend,
                })
              }
            >
              {confirming ? t("setup.generating") : t("setup.generate")}
            </Button>
          ) : (
            <img
              alt={t("setup.preview")}
              className="w-64 rounded-md border border-zinc-200 dark:border-zinc-800"
              src={`data:image/png;base64,${image}`}
            />
          )}
          {outcome !== null && outcome !== "passed" ? (
            <p role="alert" className="text-sm text-red-700 dark:text-red-300">
              {t(`setup.outcome.${outcome}`, { detail })}
            </p>
          ) : null}
        </section>
      ) : null}
      <div>
        <Button disabled={image === null} onClick={onContinue}>
          {t("setup.continue")}
        </Button>
      </div>
    </main>
  );
}
