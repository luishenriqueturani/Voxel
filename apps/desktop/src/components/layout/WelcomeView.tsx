import { useTranslation } from "react-i18next";
import type { SetupStep } from "../../native/setup";
import { Button } from "../ui/button";

type WelcomeViewProps = {
  steps: SetupStep[];
  onContinue: () => void;
};

export function WelcomeView({ steps, onContinue }: WelcomeViewProps) {
  const { t } = useTranslation();
  const finished = steps.some((step) => step.id === "profile" && step.state === "done");
  const labelFor = (step: SetupStep) => {
    if (step.id === "hardware") {
      return t("setup.hardware", { detail: step.detail });
    }
    if (step.id === "profile") {
      return t("setup.profile", { profile: t(`setup.profiles.${step.detail}`) });
    }
    return t("setup.backend", { backend: t(`setup.backends.${step.id}`) });
  };

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
      <div>
        <Button disabled={!finished} onClick={onContinue}>
          {t("setup.continue")}
        </Button>
      </div>
    </main>
  );
}
