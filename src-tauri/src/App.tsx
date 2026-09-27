import React, { useState } from 'react';
import './App.css';
import { ConfirmationFlow, BlockDevice } from './components/ConfirmationFlow';

export type AppScreen =
  | 'SELECTION'
  | 'CONFIRMATION'
  | 'CLONING'
  | 'VERIFICATION'
  | 'COMPLETE'
  | 'RECOVERY'
  | 'ERROR';

export interface RecoveryCheckpoint {
  sourceSerial: string;
  destSerial: string;
  totalBytes: number;
  resumedOffset: number;
  percentComplete: number;
}

export const App: React.FC = () => {
  const [currentScreen, setCurrentScreen] = useState<AppScreen>('SELECTION');
  const [source, setSource] = useState<BlockDevice | null>(null);
  const [destination, setDestination] = useState<BlockDevice | null>(null);
  const [showConfirmationFlow, setShowConfirmationFlow] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [recoveryCheckpoint, setRecoveryCheckpoint] = useState<RecoveryCheckpoint | null>(null);

  const handleStartConfirmation = () => {
    if (!source || !destination) {
      setError('Please select both a source device and destination device.');
      return;
    }
    if (destination.isSystemDisk) {
      setError('Safety Violation: Destination is the host operating system disk.');
      return;
    }
    if (destination.isMounted) {
      setError('Safety Violation: Destination contains active mounted partitions.');
      return;
    }
    setShowConfirmationFlow(true);
    setCurrentScreen('CONFIRMATION');
  };

  const handleConfirmClone = () => {
    setShowConfirmationFlow(false);
    setCurrentScreen('CLONING');
  };

  const handleCancelConfirmation = () => {
    setShowConfirmationFlow(false);
    setCurrentScreen('SELECTION');
  };

  const handleResumeCheckpoint = () => {
    if (!recoveryCheckpoint) return;
    setCurrentScreen('CLONING');
  };

  const handleDismissCheckpoint = () => {
    setRecoveryCheckpoint(null);
  };

  return (
    <div className="app-container">
      {/* Top Header Bar */}
      <header className="header-bar">
        <div>
          <h1 className="brand-title">DiskClone</h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', margin: '4px 0 0 0' }}>
            High-Performance Safety-Critical Linux Disk Cloning & Recovery
          </p>
        </div>
        <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
          <span style={{ fontSize: '0.85rem', color: 'var(--text-muted)' }}>
            Screen: <strong style={{ color: 'var(--text-primary)' }}>{currentScreen}</strong>
          </span>
        </div>
      </header>

      {/* Recovery Checkpoint Banner */}
      {recoveryCheckpoint && currentScreen === 'SELECTION' && (
        <div className="recovery-banner">
          <div>
            <strong>Interrupted Clone Checkpoint Detected:</strong> Resumable at{' '}
            {recoveryCheckpoint.percentComplete.toFixed(1)}% ({recoveryCheckpoint.sourceSerial} →{' '}
            {recoveryCheckpoint.destSerial})
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button onClick={handleResumeCheckpoint} className="btn-primary" style={{ padding: '6px 14px' }}>
              Resume Clone
            </button>
            <button onClick={handleDismissCheckpoint} className="btn-secondary" style={{ padding: '6px 14px' }}>
              Dismiss
            </button>
          </div>
        </div>
      )}

      {/* Error Bar */}
      {error && (
        <div style={{ background: 'var(--danger-bg)', border: '1px solid var(--danger-color)', padding: 12, borderRadius: 8, color: '#fca5a5', display: 'flex', justifyContent: 'space-between' }}>
          <span>{error}</span>
          <button onClick={() => setError(null)} style={{ background: 'none', border: 'none', color: '#fff', cursor: 'pointer' }}>✕</button>
        </div>
      )}

      {/* Main Screen Router */}
      <main style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
        {currentScreen === 'SELECTION' && (
          <div style={{ textAlign: 'center', padding: '40px 20px', background: 'var(--bg-secondary)', borderRadius: 12, border: '1px solid var(--border-color)' }}>
            <h2>Disk Clone Workspace Ready</h2>
            <p style={{ color: 'var(--text-secondary)' }}>Select source and destination devices to proceed.</p>
            {source && destination && (
              <button
                onClick={handleStartConfirmation}
                className="btn-primary"
                style={{ fontSize: '1.1rem', padding: '14px 40px', marginTop: 20 }}
              >
                Initiate Multi-Step Safety Confirmation →
              </button>
            )}
          </div>
        )}

        {currentScreen === 'CLONING' && (
          <div style={{ textAlign: 'center', padding: '40px 20px', background: 'var(--bg-secondary)', borderRadius: 12, border: '1px solid var(--border-color)' }}>
            <h2>Cloning In Progress...</h2>
            <div style={{ display: 'flex', justifyContent: 'center', marginTop: 16 }}>
              <button onClick={() => setCurrentScreen('SELECTION')} className="btn-secondary">
                Cancel Operation
              </button>
            </div>
          </div>
        )}
      </main>

      {/* Multi-Step Confirmation Modal */}
      {source && destination && (
        <ConfirmationFlow
          source={source}
          destination={destination}
          isOpen={showConfirmationFlow}
          onConfirm={handleConfirmClone}
          onCancel={handleCancelConfirmation}
        />
      )}
    </div>
  );
};

export default App;
