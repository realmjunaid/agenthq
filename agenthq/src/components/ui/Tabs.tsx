import Button from "./Button";

export default function Tabs({
  tabs,
  active,
  onChange,
}: {
  tabs: { id: string; label: string }[];
  active: string;
  onChange: (id: string) => void;
}) {
  return (
    <div className="tabs" role="tablist">
      {tabs.map((t) => (
        <Button
          key={t.id}
          variant="ghost"
          role="tab"
          aria-selected={active === t.id}
          className={active === t.id ? "tab-active" : ""}
          onClick={() => onChange(t.id)}
        >
          {t.label}
        </Button>
      ))}
    </div>
  );
}
