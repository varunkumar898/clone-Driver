import { useState } from 'react';
import { ProgressUpdate } from '../types/clone';

export function useCloneProgress() {
  const [progress, setProgress] = useState<ProgressUpdate>({
    bytesProcessed: 0,
    totalBytes: 0,
    speedBytesPerSec: 0,
    percent: 0,
    etaSeconds: 0,
    currentChunk: 0,
    totalChunks: 0,
  });

  return { progress, setProgress };
}
