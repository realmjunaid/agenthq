import type { ReactNode } from "react";

export default function Card({
  title,
  actions,
  className = "",
  children,
}: {
  title?: string;
  actions?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={`card${className ? ` ${className}` : ""}`}>
      {(title !== undefined || actions !== undefined) && (
        <div className="card-header">
          {title !== undefined && <h3 className="card-title">{title}</h3>}
          {actions}
        </div>
      )}
      <div className="card-body">{children}</div>
    </div>
  );
}
