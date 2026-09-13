import React from 'react';
import { ProgressUpdate } from '../types/clone';
import { formatBytes, formatDuration, formatSpeed } from '../utils/formatting';

interface CloneProgressProps {
  progress: ProgressUpdate;
}

export const CloneProgress: React.FC<CloneProgressProps> = ({ progress }) => {
  return (
    <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <h3>Cloning Operation</h3>
        <span style={{ fontFamily: 'var(--font-mono)', fontSize: '1.2rem', fontWeight: 700 }}>
          {progress.percent.toFixed(1)}%
        </span>
      </div>

      <div style={{ height: 10, background: 'var(--bg-tertiary)', borderRadius: 5, overflow: 'hidden' }}>
        <div
          style={{
            height: '100%',
            width: `${Math.min(100, Math.max(0, progress.percent))}%`,
            background: 'linear-gradient(90deg, var(--accent-cyan), var(--accent-blue))',
            transition: 'width 0.3s ease',
          }}
        />
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 12 }}>
        <div>
          <div style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>TRANSFERRED</div>
          <div style={{ fontWeight: 600 }}>{formatBytes(progress.bytesProcessed)} / {formatBytes(progress.totalBytes)}</div>
        </div>
        <div>
          <div style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>SPEED</div>
          <div style={{ fontWeight: 600 }}>{formatSpeed(progress.speedBytesPerSec)}</div>
        </div>
        <div>
          <div style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>ESTIMATED TIME</div>
          <div style={{ fontWeight: 600 }}>{formatDuration(progress.etaSeconds)}</div>
        </div>
        <div>
          <div style={{ color: 'var(--text-muted)', fontSize: '0.8rem' }}>CHUNKS</div>
          <div style={{ fontWeight: 600 }}>{progress.currentChunk} / {progress.totalChunks}</div>
        </div>
      </div>
    </div>
  );
};
