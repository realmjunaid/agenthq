import type { ReactNode } from "react";

export default function Tooltip({
  tip,
  children,
}: {
  tip: string;
  children: ReactNode;
}) {
  return (
    <span className="tooltip" tabIndex={0}>
      {children}
      <span className="tooltip-tip" role="tooltip">
        {tip}
      </span>
    </span>
  );
}
