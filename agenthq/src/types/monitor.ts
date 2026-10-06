export type DiskStat = {
  name: string;
  mount: string;
  total_bytes: number;
  available_bytes: number;
};

export type NetStat = {
  name: string;
  received_bytes: number;
  transmitted_bytes: number;
};

export type HostView = {
  cpu: number;
  used_mem_bytes: number;
  total_mem_bytes: number;
  disks: DiskStat[];
  networks: NetStat[];
  paused: boolean;
};
