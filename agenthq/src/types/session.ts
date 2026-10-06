// Mirrors of serialized Rust Session/Project (sessions engine).
export type SessionConfidence = "confirmed" | "detected" | "estimated" | "unknown";

export type Session = {
  id: string;
  agent_id: string;
  project_id: string | null;
  status: string;
  model: string | null;
  started_at: number | null;
  last_activity: number | null;
  confidence: SessionConfidence;
};

export type Project = {
  id: string;
  name: string;
  path: string;
  last_seen: number | null;
  agent_ids: string[];
};
