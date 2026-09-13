export type CloneState =
  | 'IDLE'
  | 'SCANNING_DEVICES'
  | 'AWAITING_SOURCE_SELECTION'
  | 'AWAITING_DESTINATION_SELECTION'
  | 'VALIDATING_SELECTION'
  | 'AWAITING_CONFIRMATION'
  | 'PREPARING'
  | 'CLONING'
  | 'FLUSH'
  | 'VERIFYING'
  | 'COMPLETED'
  | 'CANCELLING'
  | 'ERROR';

export interface ProgressUpdate {
  bytesProcessed: number;
  totalBytes: number;
  speedBytesPerSec: number;
  percent: number;
  etaSeconds: number;
  currentChunk: number;
  totalChunks: number;
}

export interface VerificationSummary {
  passed: boolean;
  totalBlocksVerified: number;
  matchedBlocks: number;
  mismatchedBlocks: number;
  sourceHash: string;
  destHash: string;
}
