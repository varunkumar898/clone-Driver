import { BlockDevice } from './device';
import { VerificationSummary } from './clone';

export interface ScanDevicesResponse {
  devices: BlockDevice[];
}

export interface StartCloneRequest {
  sourcePath: string;
  destPath: string;
  confirmationToken: string;
}

export interface LogEntryPayload {
  timestamp: string;
  level: 'DEBUG' | 'INFO' | 'WARN' | 'ERROR';
  target: string;
  message: string;
}
