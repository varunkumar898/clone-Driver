import React, { useState } from 'react';
import { LogEntryPayload } from '../types/ipc';

interface LogViewerProps {
  logs: LogEntryPayload[];
}

export const LogViewer: React.FC<LogViewerProps> = ({ logs }) => {
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <div className="card" style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
      <div
        onClick={() => setIsExpanded(!isExpanded)}
        style={{ display: 'flex', justifyContent: 'space-between', cursor: 'pointer' }}
      >
        <h4 style={{ color: 'var(--text-secondary)' }}>System Logs ({logs.length})</h4>
        <span style={{ color: 'var(--accent-blue)', fontSize: '0.85rem' }}>
          {isExpanded ? 'Collapse' : 'Expand'}
        </span>
      </div>

      {isExpanded && (
        <div
          style={{
            maxHeight: 200,
            overflowY: 'auto',
            background: 'var(--bg-primary)',
            padding: 10,
            borderRadius: 6,
            fontFamily: 'var(--font-mono)',
            fontSize: '0.8rem',
            display: 'flex',
            flexDirection: 'column',
            gap: 4,
          }}
        >
          {logs.length === 0 ? (
            <div style={{ color: 'var(--text-muted)' }}>No logs recorded.</div>
          ) : (
            logs.map((l, i) => (
              <div key={i}>
                <span style={{ color: 'var(--text-muted)' }}>[{l.timestamp}] </span>
                <span
                  style={{
                    color:
                      l.level === 'ERROR'
                        ? 'var(--accent-danger)'
                        : l.level === 'WARN'
                        ? 'var(--accent-warning)'
                        : 'var(--accent-cyan)',
                  }}
                >
                  {l.level}{' '}
                </span>
                <span>{l.message}</span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};
