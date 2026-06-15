import { Check } from "lucide-react";
import { useEffect, useState } from "react";

interface ToastProps {
  message: string | null;
}

export function Toast({ message }: ToastProps) {
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
      <Check aria-hidden="true" />
      <span>{visibleMessage}</span>
    </div>
  );
}
