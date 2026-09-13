export interface PartitionInfo {
  index: number;
  node: string;
  startSector: number;
  lengthSectors: number;
  sizeBytes: number;
  filesystem?: string;
  mountPoint?: string;
  isSystemPartition: boolean;
}

export interface BlockDevice {
  path: string;
  name: string;
  model: string;
  vendor: string;
  serialNumber: string;
  sizeBytes: number;
  sectorSize: number;
  isRotational: boolean;
  isSystemDisk: boolean;
  isMounted: boolean;
  partitions: PartitionInfo[];
}
