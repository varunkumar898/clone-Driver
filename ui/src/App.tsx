import React, { useState } from 'react';
import './styles/globals.css';
import './styles/components.css';
import { DiskSelector } from './components/DiskSelector';
import { CloneProgress } from './components/CloneProgress';
import { ConfirmationDialog } from './components/ConfirmationDialog';
import { ErrorDisplay } from './components/ErrorDisplay';
import { LogViewer } from './components/LogViewer';
import { VerificationResult } from './components/VerificationResult';
import { useDeviceList } from './hooks/useDeviceList';
import { useCloneProgress } from './hooks/useCloneProgress';
import { useCloneStatus } from './hooks/useCloneStatus';
import { BlockDevice } from './types/device';
import { LogEntryPayload } from './types/ipc';
import { VerificationSummary } from './types/clone';

export const App: React.FC = () => {
  const { devices, loading, refreshDevices } = useDeviceList();
  const { progress } = useCloneProgress();
  const { state, setState, statusMessage } = useCloneStatus();

  const [source, setSource] = useState<BlockDevice | null>(null);
  const [destination, setDestination] = useState<BlockDevice | null>(null);
  const [showConfirm, setShowConfirm] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [logs] = useState<LogEntryPayload[]>([]);
  const [verificationSummary, setVerificationSummary] = useState<VerificationSummary | null>(null);

  const handleStartClone = () => {
    if (!source || !destination) return;
    setShowConfirm(true);
  };

  const handleConfirmClone = () => {
    setShowConfirm(false);
    setState('CLONING');
  };

  return (
    <div className="app-container">
      <header className="header-bar">
        <div>
          <h1 className="brand-title">DiskClone</h1>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
            High-Performance Safety-Critical Linux Disk Cloning
          </p>
        </div>
        <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
          <span style={{ fontSize: '0.85rem', color: 'var(--text-muted)' }}>Status: {statusMessage}</span>
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

      <ErrorDisplay error={error} onDismiss={() => setError(null)} />

      <main style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
        <div style={{ display: 'flex', gap: 20 }}>
          <DiskSelector
            title="1. Source Device (Read-Only)"
            devices={devices}
            selectedPath={source?.path}
            onSelect={setSource}
            disabled={state === 'CLONING'}
          />
          <DiskSelector
            title="2. Destination Device (Target)"
            devices={devices.filter((d) => d.path !== source?.path)}
            selectedPath={destination?.path}
            onSelect={setDestination}
            disabled={state === 'CLONING'}
          />
        </div>

        {source && destination && state === 'IDLE' && (
          <div style={{ alignSelf: 'center' }}>
            <button onClick={handleStartClone} className="btn-primary" style={{ fontSize: '1.1rem', padding: '12px 32px' }}>
              Initiate Clone Sequence
            </button>
          </div>
        )}

        {state === 'CLONING' && <CloneProgress progress={progress} />}

        {verificationSummary && (
          <VerificationResult
            summary={verificationSummary}
            onClose={() => setVerificationSummary(null)}
          />
        )}

        <LogViewer logs={logs} />
      </main>

      {source && destination && (
        <ConfirmationDialog
          source={source}
          destination={destination}
          isOpen={showConfirm}
          onConfirm={handleConfirmClone}
          onCancel={() => setShowConfirm(false)}
        />
      )}
    </div>
  );
};

export default App;
