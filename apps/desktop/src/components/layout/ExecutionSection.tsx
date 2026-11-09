import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { type QueueFinish, loadStoredSettings, saveStoredSettings } from "../../native/settings";

const FINISH: QueueFinish[] = ["none", "notify", "suspend", "poweroff"];

export function ExecutionSection() {
  const { t } = useTranslation();
  const [inhibitSleep, setInhibitSleep] = useState(true);
  const [queueFinish, setQueueFinish] = useState<QueueFinish>("none");

  useEffect(() => {
    void loadStoredSettings().then((stored) => {
      if (stored === null) {
        return;
      }
      setInhibitSleep(stored.inhibitSleep ?? true);
      setQueueFinish(stored.queueFinish ?? "none");
    });
  }, []);

  return (
    <div className="flex max-w-sm flex-col gap-4 text-sm">
      <label className="flex items-start gap-2">
        <input
          type="checkbox"
          checked={inhibitSleep}
          onChange={(event) => {
            const next = event.target.checked;
            setInhibitSleep(next);
            void saveStoredSettings({ inhibitSleep: next });
          }}
        />
        <span>
          {t("settings.inhibitSleep")}
          <span className="mt-1 block text-zinc-500">{t("settings.inhibitSleepHint")}</span>
        </span>
      </label>
      <label className="flex flex-col gap-1">
        {t("settings.queueFinish")}
        <select
          value={queueFinish}
          onChange={(event) => {
            const next = event.target.value as QueueFinish;
            setQueueFinish(next);
            void saveStoredSettings({ queueFinish: next });
          }}
          className="rounded-md border border-zinc-300 bg-white px-2 py-1.5 dark:border-zinc-700 dark:bg-zinc-950"
        >
          {FINISH.map((id) => (
            <option key={id} value={id}>
              {t(`settings.queueFinishOptions.${id}`)}
            </option>
          ))}
        </select>
      </label>
    </div>
  );
}
