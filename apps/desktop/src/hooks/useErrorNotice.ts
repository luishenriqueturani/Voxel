import { useEffect, useRef } from "react";
import { type ErrorNotice, listenToErrors } from "../native/errors";

export function useErrorNotice(onNotice: (notice: ErrorNotice) => void): void {
  const onNoticeRef = useRef(onNotice);
  onNoticeRef.current = onNotice;

  useEffect(() => {
    let stop: (() => void) | undefined;
    let cancelled = false;
    void listenToErrors((notice) => onNoticeRef.current(notice)).then((unlisten) => {
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
  }, []);
}
