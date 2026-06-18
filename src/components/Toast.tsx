import { Check, CircleAlert, Info } from "lucide-react";
import { useEffect, useState } from "react";
import type { ToastKind } from "../hooks/useToast";

interface ToastProps {
  icon?: boolean;
  kind?: ToastKind;
  message: string | null;
}

function getToastIcon(kind: ToastKind) {
  if (kind === "error") {
    return <CircleAlert aria-hidden="true" />;
  }

  if (kind === "info") {
    return <Info aria-hidden="true" />;
  }

  return <Check aria-hidden="true" />;
}

export function Toast({ icon = true, kind = "success", message }: ToastProps) {
  const [visibleIcon, setVisibleIcon] = useState(icon);
  const [visibleKind, setVisibleKind] = useState(kind);
  const [visibleMessage, setVisibleMessage] = useState(message);

  useEffect(() => {
    if (message) {
      setVisibleIcon(icon);
      setVisibleKind(kind);
      setVisibleMessage(message);
    }
  }, [icon, kind, message]);

  return (
    <div
      aria-live="polite"
      className={`toast toast--${visibleKind} ${message ? "is-visible" : ""}`}
      onTransitionEnd={() => {
        if (!message) {
          setVisibleMessage(null);
        }
      }}
      role="status"
    >
      {visibleIcon ? getToastIcon(visibleKind) : null}
      <span>{visibleMessage}</span>
    </div>
  );
}
