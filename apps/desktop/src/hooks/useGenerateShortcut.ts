import { useEffect } from "react";

export function useGenerateShortcut(onGenerate: () => void): void {
  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Enter" && (event.ctrlKey || event.metaKey) && !event.repeat) {
        event.preventDefault();
        onGenerate();
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onGenerate]);
}
