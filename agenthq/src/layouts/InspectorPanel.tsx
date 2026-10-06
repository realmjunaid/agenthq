import type { ReactNode } from "react";

export default function InspectorPanel({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <aside className="inspector">
      <h3 className="inspector-title">{title}</h3>
      <div className="inspector-body">{children}</div>
    </aside>
  );
}
