import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { listenToMenu, syncNativeMenu } from "../native/menu";

export function useNativeMenu(onAction: (id: string) => void): void {
  const { i18n } = useTranslation();
  const onActionRef = useRef(onAction);
  onActionRef.current = onAction;

  useEffect(() => {
    const bundle: unknown = i18n.getResourceBundle(i18n.language, "translation");
    const menu =
      bundle !== null && typeof bundle === "object" && "menu" in bundle ? bundle.menu : null;
    if (menu !== null && typeof menu === "object") {
      void syncNativeMenu(menu as Record<string, string>);
    }

    let stop: (() => void) | undefined;
    let cancelled = false;
    void listenToMenu((id) => onActionRef.current(id)).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        stop = unlisten;
      }
    });

    return () => {
      cancelled = true;
      stop?.();
    };
  }, [i18n, i18n.language]);
}
