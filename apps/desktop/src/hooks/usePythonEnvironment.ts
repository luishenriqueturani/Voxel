import { useEffect, useState } from "react";
import {
  awaitPythonEnvironment,
  listenToPythonEnvironment,
  loadPythonEnvironment,
  note3dIntent,
  type PythonIntent,
  type PythonProgress,
  reinstallPythonEnvironment,
  removePythonEnvironment,
} from "../native/pythonEnv";

export function usePythonEnvironment() {
  const [progress, setProgress] = useState<PythonProgress | undefined>(undefined);

  useEffect(() => {
    let unlisten: () => void = () => undefined;
    let cancelled = false;
    void loadPythonEnvironment().then((value) => {
      if (!cancelled && value !== null) {
        setProgress(value);
      }
    });
    void listenToPythonEnvironment((value) => setProgress(value)).then((stop) => {
      if (cancelled) {
        stop();
        return;
      }
      unlisten = stop;
    });
    return () => {
      cancelled = true;
      unlisten();
    };
  }, []);

  return {
    progress,
    note: (reason: PythonIntent) => note3dIntent(reason),
    wait: awaitPythonEnvironment,
    reinstall: reinstallPythonEnvironment,
    remove: removePythonEnvironment,
  };
}
