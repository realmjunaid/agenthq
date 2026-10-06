// Mirrors of serialized Rust monitoring types.
export type ProcessInfo = {
  pid: number;
  parent_pid: number | null;
  name: string;
  exe: string | null;
  cpu: number;
  ram_bytes: number;
  started_at: number;
  cmdline: string | null;
};

export type SystemStats = {
  total_cpu: number;
  used_mem_bytes: number;
  total_mem_bytes: number;
};
