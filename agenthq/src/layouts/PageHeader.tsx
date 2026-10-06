import type { ReactNode } from "react";

export default function PageHeader({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle?: string;
  actions?: ReactNode;
}) {
  return (
    <div className="page-header">
      <div>
        <h2 className="page-title">{title}</h2>
        {subtitle !== undefined && (
          <div className="page-subtitle">{subtitle}</div>
        )}
      </div>
      {actions}
    </div>
  );
}
