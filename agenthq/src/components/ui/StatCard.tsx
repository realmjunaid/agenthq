import Card from "./Card";
import Metric from "./Metric";
import Badge, { type BadgeTone } from "./Badge";

export default function StatCard({
  label,
  value,
  delta,
  tone = "neutral",
}: {
  label: string;
  value: string;
  delta?: string;
  tone?: BadgeTone;
}) {
  return (
    <Card>
      <Metric label={label} value={value} />
      {delta !== undefined && <Badge tone={tone}>{delta}</Badge>}
    </Card>
  );
}
