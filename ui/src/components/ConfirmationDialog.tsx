import React, { useState } from 'react';
import { BlockDevice } from '../types/device';
import { REQUIRED_CONFIRMATION_TEXT, isConfirmationValid } from '../utils/validation';

interface ConfirmationDialogProps {
  source: BlockDevice;
  destination: BlockDevice;
  isOpen: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export const ConfirmationDialog: React.FC<ConfirmationDialogProps> = ({
  source,
  destination,
  isOpen,
  onConfirm,
  onCancel,
}) => {
  const [typedConfirmation, setTypedConfirmation] = useState('');

  if (!isOpen) return null;

  const canProceed = isConfirmationValid(typedConfirmation);

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.75)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
    >
      <div className="card" style={{ maxWidth: 520, width: '100%', borderColor: 'var(--accent-danger)' }}>
        <h2 style={{ color: 'var(--accent-danger)', marginBottom: 12 }}>CRITICAL WARNING</h2>
        <p style={{ color: 'var(--text-secondary)', marginBottom: 16 }}>
          All data on destination device <strong>{destination.path}</strong> ({destination.model || destination.name})
          will be <strong>PERMANENTLY ERASED</strong> and overwritten with data from <strong>{source.path}</strong>.
        </p>

        <div style={{ background: 'var(--bg-tertiary)', padding: 12, borderRadius: 8, marginBottom: 16 }}>
          <div style={{ fontSize: '0.9rem' }}><strong>Source S/N:</strong> {source.serialNumber || 'N/A'}</div>
          <div style={{ fontSize: '0.9rem' }}><strong>Destination S/N:</strong> {destination.serialNumber || 'N/A'}</div>
        </div>

        <p style={{ fontSize: '0.85rem', marginBottom: 8 }}>
          To confirm, type exactly: <code>{REQUIRED_CONFIRMATION_TEXT}</code>
        </p>

        <input
          type="text"
          value={typedConfirmation}
          onChange={(e) => setTypedConfirmation(e.target.value)}
          placeholder={REQUIRED_CONFIRMATION_TEXT}
          style={{
            width: '100%',
            padding: 10,
            background: 'var(--bg-primary)',
            border: '1px solid var(--border-subtle)',
            borderRadius: 6,
            color: 'var(--text-primary)',
            fontFamily: 'var(--font-mono)',
            marginBottom: 20,
          }}
        />

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 12 }}>
          <button
            onClick={onCancel}
            style={{
              padding: '8px 16px',
              borderRadius: 6,
              background: 'transparent',
              color: 'var(--text-secondary)',
              border: '1px solid var(--border-subtle)',
              cursor: 'pointer',
            }}
          >
            Cancel
          </button>
          <button
            onClick={onConfirm}
            disabled={!canProceed}
            style={{
              padding: '8px 16px',
              borderRadius: 6,
              background: canProceed ? 'var(--accent-danger)' : 'var(--bg-tertiary)',
              color: canProceed ? 'white' : 'var(--text-muted)',
              border: 'none',
              cursor: canProceed ? 'pointer' : 'not-allowed',
              fontWeight: 600,
            }}
          >
            Erase & Clone
          </button>
        </div>
      </div>
    </div>
  );
};
