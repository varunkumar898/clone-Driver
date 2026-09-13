import React from 'react';

interface ErrorDisplayProps {
  error: string | null;
  onDismiss: () => void;
  onRetry?: () => void;
}

export const ErrorDisplay: React.FC<ErrorDisplayProps> = ({ error, onDismiss, onRetry }) => {
  if (!error) return null;

  return (
    <div
      className="card"
      style={{
        borderColor: 'var(--accent-danger)',
        background: 'rgba(239, 68, 68, 0.1)',
        display: 'flex',
        flexDirection: 'column',
        gap: 12,
      }}
    >
      <div style={{ color: 'var(--accent-danger)', fontWeight: 700 }}>Operation Error</div>
      <div style={{ color: 'var(--text-primary)', fontFamily: 'var(--font-mono)', fontSize: '0.9rem' }}>
        {error}
      </div>
      <div style={{ display: 'flex', gap: 10, alignSelf: 'flex-end' }}>
        {onRetry && (
          <button
            onClick={onRetry}
            style={{
              padding: '6px 14px',
              borderRadius: 6,
              background: 'var(--bg-secondary)',
              color: 'var(--text-primary)',
              border: '1px solid var(--border-subtle)',
              cursor: 'pointer',
            }}
          >
            Retry
          </button>
        )}
        <button
          onClick={onDismiss}
          style={{
            padding: '6px 14px',
            borderRadius: 6,
            background: 'var(--accent-danger)',
            color: 'white',
            border: 'none',
            cursor: 'pointer',
          }}
        >
          Dismiss
        </button>
      </div>
    </div>
  );
};
