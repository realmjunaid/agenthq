// Mirrors of serialized Rust Model + Connection (models engine).
// Connection rows hold key NAMES only — values are never collected.

export type Model = {
  id: string;
  name: string;
  provider: string | null;
  agent_id: string;
};

export type Connection = {
  id: string;
  name: string;
  provider: string;
  configured: boolean;
  agent_id: string;
};
