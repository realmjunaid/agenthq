import Button from "./Button";

export function EmptyState({
  title,
  hint,
  action,
}: {
  title: string;
  hint?: string;
  action?: { label: string; onClick: () => void };
}) {
  return (
    <div className="state">
      <div className="state-title">{title}</div>
      {hint !== undefined && <div className="state-hint">{hint}</div>}
      {action !== undefined && (
        <Button variant="secondary" onClick={action.onClick}>
          {action.label}
        </Button>
      )}
    </div>
  );
}

export function ErrorState({
  message,
  onRetry,
}: {
  message: string;
  onRetry?: () => void;
}) {
  return (
    <div className="state state-error">
      <div className="state-title">Something went wrong</div>
      <div className="state-hint">{message}</div>
      {onRetry !== undefined && (
        <Button variant="secondary" onClick={onRetry}>
          Retry
        </Button>
      )}
    </div>
  );
}

export function UnsupportedState({ feature }: { feature: string }) {
  return (
    <div className="state">
      <div className="state-title">Unavailable</div>
      <div className="state-hint">
        {feature} is not supported by this agent.
      </div>
    </div>
  );
}
