// Mirror of serialized Rust Plugin (plugins engine).
export type Plugin = {
  id: string;
  name: string;
  version: string | null;
  path: string | null;
  source: string | null;
  enabled: boolean;
  agent_id: string;
};
