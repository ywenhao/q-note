import type { ButtonHTMLAttributes, ReactNode } from "react";

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  active?: boolean;
  badge?: boolean;
  icon: ReactNode;
  label: string;
  subtle?: boolean;
}

export function IconButton({
  active = false,
  badge = false,
  children,
  className = "",
  icon,
  label,
  subtle = false,
  type = "button",
  ...props
}: IconButtonProps) {
  return (
    <button
      aria-label={label}
      className={`icon-button ${active ? "is-active" : ""} ${badge ? "is-badged" : ""} ${subtle ? "is-subtle" : ""} ${className}`}
      title={label}
      type={type}
      {...props}
    >
      {icon}
      {children ? <span>{children}</span> : null}
    </button>
  );
}
