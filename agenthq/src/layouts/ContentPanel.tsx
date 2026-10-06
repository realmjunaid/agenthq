import type { ReactNode } from "react";

export default function ContentPanel({
  children,
}: {
  children: ReactNode;
}) {
  return <div className="content-panel">{children}</div>;
}
