interface QMarkProps {
  className?: string;
}

export function QMark({ className = "" }: QMarkProps) {
  return (
    <span aria-hidden="true" className={`q-mark ${className}`}>
      <svg role="img" viewBox="0 0 100 100">
        <circle className="q-mark__background" cx="50" cy="50" r="48" />
        <path
          className="q-mark__letter"
          d="M50 23C33.5 23 23 34.5 23 50.3C23 66.4 34.5 77 50.4 77C56.6 77 62.1 75.4 66.5 72.6L76.8 80.6L84.5 69.4L74.4 63.4C76.1 59.5 77 55.1 77 50.3C77 34.5 66.5 23 50 23ZM50.2 35.8C58.9 35.8 64 41.8 64 50.3C64 58.9 58.9 64.4 50.2 64.4C41.3 64.4 36 58.9 36 50.3C36 41.8 41.3 35.8 50.2 35.8Z"
        />
      </svg>
    </span>
  );
}
