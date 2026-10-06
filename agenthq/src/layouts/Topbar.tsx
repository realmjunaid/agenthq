export default function Topbar({
  theme,
  onToggleTheme,
}: {
  theme: "light" | "dark";
  onToggleTheme: () => void;
}) {
  return (
    <header className="topbar">
      <span>AgentHQ</span>
      <button
        onClick={onToggleTheme}
        aria-label={theme === "light" ? "Switch to dark theme" : "Switch to light theme"}
      >
        {theme === "light" ? "Dark" : "Light"}
      </button>
    </header>
  );
}
