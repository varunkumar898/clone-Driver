import React from 'react';
import { BlockDevice } from '../types/device';
import { formatBytes } from '../utils/formatting';

interface DiskSelectorProps {
  title: string;
  devices: BlockDevice[];
  selectedPath?: string;
  onSelect: (device: BlockDevice) => void;
  disabled?: boolean;
}

export const DiskSelector: React.FC<DiskSelectorProps> = ({
  title,
  devices,
  selectedPath,
  onSelect,
  disabled,
}) => {
  return (
    <div className="card" style={{ flex: 1 }}>
      <h3 style={{ marginBottom: 12, color: 'var(--text-primary)' }}>{title}</h3>
      {devices.length === 0 ? (
        <p style={{ color: 'var(--text-muted)' }}>No candidate devices found.</p>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {devices.map((d) => {
            const isSelected = d.path === selectedPath;
            return (
              <div
                key={d.path}
                onClick={() => !disabled && onSelect(d)}
                style={{
                  border: isSelected ? '1px solid var(--accent-blue)' : '1px solid var(--border-subtle)',
                  borderRadius: 8,
                  padding: 12,
                  cursor: disabled ? 'not-allowed' : 'pointer',
                  backgroundColor: isSelected ? 'var(--bg-tertiary)' : 'transparent',
                }}
              >
                <div style={{ fontWeight: 600 }}>{d.model || d.name} ({formatBytes(d.sizeBytes)})</div>
                <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                  {d.path} • S/N: {d.serialNumber || 'N/A'}
                </div>
                {d.isSystemDisk && (
                  <span style={{ color: 'var(--accent-danger)', fontSize: '0.75rem', fontWeight: 600 }}>
                    [SYSTEM DISK]
                  </span>
                )}
                {d.isMounted && (
                  <span style={{ color: 'var(--accent-warning)', fontSize: '0.75rem', marginLeft: 8 }}>
                    [MOUNTED]
                  </span>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
