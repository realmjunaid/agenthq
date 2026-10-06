import type { ReactNode } from "react";

export type BadgeTone = "neutral" | "success" | "warning" | "error" | "info";

const TONES: Record<BadgeTone, string> = {
  neutral: "badge-neutral",
  success: "badge-success",
  warning: "badge-warning",
  error: "badge-error",
  info: "badge-info",
};

export default function Badge({
  tone = "neutral",
  className = "",
  children,
}: {
  tone?: BadgeTone;
  className?: string;
  children: ReactNode;
}) {
  return (
    <span className={`badge ${TONES[tone]}${className ? ` ${className}` : ""}`}>
      {children}
    </span>
  );
}
