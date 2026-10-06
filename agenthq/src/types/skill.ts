// Mirror of serialized Rust Skill (skills engine).
export type SkillScope = "global" | "project";

export type Skill = {
  id: string;
  name: string;
  description: string | null;
  path: string | null;
  scope: SkillScope;
  source: string | null;
  agent_id: string;
};
