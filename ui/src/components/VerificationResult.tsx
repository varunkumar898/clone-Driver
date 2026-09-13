import React from 'react';
import { VerificationSummary } from '../types/clone';

interface VerificationResultProps {
  summary: VerificationSummary;
  onClose: () => void;
}

export const VerificationResult: React.FC<VerificationResultProps> = ({ summary, onClose }) => {
  return (
    <div
      className="card"
      style={{
        borderColor: summary.passed ? 'var(--accent-success)' : 'var(--accent-danger)',
        display: 'flex',
        flexDirection: 'column',
        gap: 12,
      }}
    >
      <h3 style={{ color: summary.passed ? 'var(--accent-success)' : 'var(--accent-danger)' }}>
        {summary.passed ? '✓ Clone Verification Passed' : '✗ Verification Failed'}
      </h3>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8, fontSize: '0.9rem' }}>
        <div>Total Blocks: {summary.totalBlocksVerified}</div>
        <div>Matched Blocks: {summary.matchedBlocks}</div>
        <div>Mismatches: {summary.mismatchedBlocks}</div>
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: '0.75rem', gridColumn: 'span 2' }}>
          Hash: {summary.destHash || 'N/A'}
        </div>
      </div>

      <button
        onClick={onClose}
        style={{
          alignSelf: 'flex-end',
          padding: '6px 16px',
          background: 'var(--bg-tertiary)',
          border: '1px solid var(--border-subtle)',
          borderRadius: 6,
          color: 'var(--text-primary)',
          cursor: 'pointer',
        }}
      >
        Close
      </button>
    </div>
  );
};
