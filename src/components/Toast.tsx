import { Check } from "lucide-react";
import { useEffect, useState } from "react";

interface ToastProps {
  icon?: boolean;
  message: string | null;
}

export function Toast({ icon = true, message }: ToastProps) {
  const [visibleMessage, setVisibleMessage] = useState(message);

  useEffect(() => {
    if (message) {
      setVisibleMessage(message);
    }
  }, [message]);

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
      {icon ? <Check aria-hidden="true" /> : null}
      <span>{visibleMessage}</span>
    </div>
  );
}
