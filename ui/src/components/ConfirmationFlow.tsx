import React, { useState, useEffect } from 'react';
import { BlockDevice } from '../types/device';

export interface ConfirmationFlowProps {
  source: BlockDevice;
  destination: BlockDevice;
  isOpen: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export const REQUIRED_CONFIRMATION_TOKEN = 'CLONE TO THIS DISK';

export const ConfirmationFlow: React.FC<ConfirmationFlowProps> = ({
  source,
  destination,
  isOpen,
  onConfirm,
  onCancel,
}) => {
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [typedToken, setTypedToken] = useState<string>('');
  const [hasAcknowledgedLoss, setHasAcknowledgedLoss] = useState<boolean>(false);
  const [hasVerifiedTarget, setHasVerifiedTarget] = useState<boolean>(false);

  useEffect(() => {
    if (isOpen) {
      setStep(1);
      setTypedToken('');
      setHasAcknowledgedLoss(false);
      setHasVerifiedTarget(false);
    }
  }, [isOpen]);

  if (!isOpen) return null;

  const isTokenMatching = typedToken === REQUIRED_CONFIRMATION_TOKEN;
  const formatBytes = (bytes: number): string => {
    const gb = bytes / (1000 * 1000 * 1000);
    return `${gb.toFixed(1)} GB`;
  };

  return (
    <div className="confirmation-modal-overlay" role="dialog" aria-modal="true">
      <div className="confirmation-modal-card">
        {/* Step Indicator */}
        <div className="step-indicator-bar">
          <div className={`step-dot ${step >= 1 ? 'active' : ''}`}>1. Visual Inspection</div>
          <div className="step-line" />
          <div className={`step-dot ${step >= 2 ? 'active' : ''}`}>2. Text Verification</div>
          <div className="step-line" />
          <div className={`step-dot ${step === 3 ? 'active' : ''}`}>3. Final Safety Check</div>
        </div>

        {/* Step 1: Visual Inspection */}
        {step === 1 && (
          <div className="confirmation-step-content">
            <div className="danger-header">
              <span className="warning-icon">⚠️</span>
              <h2>Confirm Disk Selection</h2>
            </div>
            <p className="step-description">
              Please visually compare the selected source and destination drives. All existing partitions
              and data on the destination device will be permanently erased.
            </p>

            <div className="device-comparison-grid">
              <div className="device-card source-card">
                <span className="card-badge badge-source">Source (Read-Only)</span>
                <h3>{source.model || source.name}</h3>
                <div className="device-meta">
                  <div className="meta-row"><span>Path:</span> <code>{source.path}</code></div>
                  <div className="meta-row"><span>Serial:</span> <code>{source.serialNumber || 'N/A'}</code></div>
                  <div className="meta-row"><span>Capacity:</span> <strong>{formatBytes(source.sizeBytes)}</strong></div>
                </div>
              </div>

              <div className="device-card dest-card">
                <span className="card-badge badge-danger">TARGET (TO BE OVERWRITTEN)</span>
                <h3>{destination.model || destination.name}</h3>
                <div className="device-meta">
                  <div className="meta-row"><span>Path:</span> <code>{destination.path}</code></div>
                  <div className="meta-row"><span>Serial:</span> <code>{destination.serialNumber || 'N/A'}</code></div>
                  <div className="meta-row"><span>Capacity:</span> <strong>{formatBytes(destination.sizeBytes)}</strong></div>
                </div>
              </div>
            </div>

            <div className="modal-actions">
              <button onClick={onCancel} className="btn-secondary">
                Cancel
              </button>
              <button onClick={() => setStep(2)} className="btn-primary">
                Proceed to Text Verification →
              </button>
            </div>
          </div>
        )}

        {/* Step 2: Text Verification */}
        {step === 2 && (
          <div className="confirmation-step-content">
            <div className="danger-header">
              <span className="warning-icon">🛑</span>
              <h2>Manual Safety Phrase Gate</h2>
            </div>
            <p className="step-description">
              To prevent accidental data destruction, type the phrase{' '}
              <strong className="token-highlight">CLONE TO THIS DISK</strong> exactly as shown below:
            </p>

            <div className="input-group">
              <input
                type="text"
                value={typedToken}
                onChange={(e) => setTypedToken(e.target.value)}
                placeholder="Type 'CLONE TO THIS DISK'"
                className={`confirmation-text-input ${isTokenMatching ? 'valid' : 'invalid'}`}
                autoFocus
              />
              <div className="validation-feedback">
                {typedToken.length === 0 ? (
                  <span className="feedback-hint">Case-sensitive verification required</span>
                ) : isTokenMatching ? (
                  <span className="feedback-success">✓ Phrase matches exactly</span>
                ) : (
                  <span className="feedback-error">✗ Characters do not match yet</span>
                )}
              </div>
            </div>

            <div className="modal-actions">
              <button onClick={() => setStep(1)} className="btn-secondary">
                ← Back
              </button>
              <button
                onClick={() => setStep(3)}
                disabled={!isTokenMatching}
                className="btn-primary"
              >
                Proceed to Final Check →
              </button>
            </div>
          </div>
        )}

        {/* Step 3: Final Safety Check */}
        {step === 3 && (
          <div className="confirmation-step-content">
            <div className="danger-header fatal-header">
              <span className="warning-icon">🚨</span>
              <h2>Final Authorization Check</h2>
            </div>
            <div className="fatal-warning-box">
              <p>
                <strong>CRITICAL WARNING:</strong> You are about to initiate raw block-level cloning to{' '}
                <code>{destination.path}</code> ({formatBytes(destination.sizeBytes)}).
                Once started, this operation is <strong>IRREVOCABLE</strong>.
              </p>
            </div>

            <div className="acknowledgement-checks">
              <label className="checkbox-row">
                <input
                  type="checkbox"
                  checked={hasAcknowledgedLoss}
                  onChange={(e) => setHasAcknowledgedLoss(e.target.checked)}
                />
                <span>I understand that all data on <code>{destination.path}</code> will be irreversibly erased.</span>
              </label>

              <label className="checkbox-row">
                <input
                  type="checkbox"
                  checked={hasVerifiedTarget}
                  onChange={(e) => setHasVerifiedTarget(e.target.checked)}
                />
                <span>I have double-checked serial numbers and verified that this is not my OS disk.</span>
              </label>
            </div>

            <div className="modal-actions">
              <button onClick={() => setStep(2)} className="btn-secondary">
                ← Back
              </button>
              <button
                onClick={onConfirm}
                disabled={!hasAcknowledgedLoss || !hasVerifiedTarget}
                className="btn-danger-execute"
              >
                Authorize & Start Clone Sequence
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
