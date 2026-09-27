import React, { useState } from 'react';
import './styles/globals.css';
import './styles/components.css';
import './App.css';
import { DiskSelector } from './components/DiskSelector';
import { CloneProgress } from './components/CloneProgress';
import { ConfirmationFlow } from './components/ConfirmationFlow';
import { ErrorDisplay } from './components/ErrorDisplay';
import { LogViewer } from './components/LogViewer';
import { VerificationResult } from './components/VerificationResult';
import { useDeviceList } from './hooks/useDeviceList';
import { useCloneProgress } from './hooks/useCloneProgress';
import { useCloneStatus } from './hooks/useCloneStatus';
import { BlockDevice } from './types/device';
import { LogEntryPayload } from './types/ipc';
import { VerificationSummary } from './types/clone';

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
  const { devices, loading, refreshDevices } = useDeviceList();
  const { progress } = useCloneProgress();
  const { state, setState, statusMessage } = useCloneStatus();

  const [currentScreen, setCurrentScreen] = useState<AppScreen>('SELECTION');
  const [source, setSource] = useState<BlockDevice | null>(null);
  const [destination, setDestination] = useState<BlockDevice | null>(null);
  const [showConfirmationFlow, setShowConfirmationFlow] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [logs] = useState<LogEntryPayload[]>([]);
  const [verificationSummary, setVerificationSummary] = useState<VerificationSummary | null>(null);
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
    setState('CLONING');
    setCurrentScreen('CLONING');
  };

  const handleCancelConfirmation = () => {
    setShowConfirmationFlow(false);
    setCurrentScreen('SELECTION');
  };

  const handleResumeCheckpoint = () => {
    if (!recoveryCheckpoint) return;
    setCurrentScreen('CLONING');
    setState('CLONING');
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
            Status: <strong style={{ color: 'var(--text-primary)' }}>{statusMessage}</strong>
          </span>
          <button
            onClick={refreshDevices}
            disabled={loading || state === 'CLONING'}
            className="btn-primary"
            style={{ fontSize: '0.85rem' }}
          >
            {loading ? 'Scanning...' : 'Scan Devices'}
          </button>
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

      {/* Error Display Bar */}
      <ErrorDisplay error={error} onDismiss={() => setError(null)} />

      {/* Main Screen Router */}
      <main style={{ display: 'flex', flexDirection: 'column', gap: 24 }}>
        {/* Screen 1: Device Selection */}
        {currentScreen === 'SELECTION' && (
          <>
            <div style={{ display: 'flex', gap: 20, flexWrap: 'wrap' }}>
              <div style={{ flex: 1, minWidth: '300px' }}>
                <DiskSelector
                  title="1. Source Device (Read-Only)"
                  devices={devices}
                  selectedPath={source?.path}
                  onSelect={setSource}
                  disabled={state === 'CLONING'}
                />
              </div>
              <div style={{ flex: 1, minWidth: '300px' }}>
                <DiskSelector
                  title="2. Destination Device (Target)"
                  devices={devices.filter((d) => d.path !== source?.path)}
                  selectedPath={destination?.path}
                  onSelect={setDestination}
                  disabled={state === 'CLONING'}
                />
              </div>
            </div>

            {source && destination && (
              <div style={{ alignSelf: 'center', marginTop: 12 }}>
                <button
                  onClick={handleStartConfirmation}
                  className="btn-primary"
                  style={{ fontSize: '1.1rem', padding: '14px 40px' }}
                >
                  Initiate Multi-Step Safety Confirmation →
                </button>
              </div>
            )}
          </>
        )}

        {/* Screen 2: Cloning Progress */}
        {currentScreen === 'CLONING' && (
          <div>
            <CloneProgress progress={progress} />
            <div style={{ display: 'flex', justifyContent: 'center', marginTop: 16 }}>
              <button
                onClick={() => {
                  setState('IDLE');
                  setCurrentScreen('SELECTION');
                }}
                className="btn-secondary"
              >
                Cancel Operation
              </button>
            </div>
          </div>
        )}

        {/* Screen 3: Verification Result */}
        {verificationSummary && (
          <VerificationResult
            summary={verificationSummary}
            onClose={() => {
              setVerificationSummary(null);
              setCurrentScreen('SELECTION');
            }}
          />
        )}

        {/* Telemetry and System Logs */}
        <LogViewer logs={logs} />
      </main>

      {/* Multi-Step Confirmation Modal (Visual → Text Input → Final Check) */}
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
