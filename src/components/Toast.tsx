import { Check } from "lucide-react";
import { useEffect, useState } from "react";

interface ToastProps {
  icon?: boolean;
  message: string | null;
}

export function Toast({ icon = true, message }: ToastProps) {
  const [visibleIcon, setVisibleIcon] = useState(icon);
  const [visibleMessage, setVisibleMessage] = useState(message);

  useEffect(() => {
    if (message) {
      setVisibleIcon(icon);
      setVisibleMessage(message);
    }
  }, [icon, message]);

  return (
    <div
      aria-live="polite"
      className={`toast ${message ? "is-visible" : ""}`}
      onTransitionEnd={() => {
        if (!message) {
          setVisibleMessage(null);
        }
      }}
      role="status"
    >
      {visibleIcon ? <Check aria-hidden="true" /> : null}
      <span>{visibleMessage}</span>
    </div>
  );
}
