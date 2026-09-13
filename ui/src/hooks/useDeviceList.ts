import { useState, useEffect, useCallback } from 'react';
import { BlockDevice } from '../types/device';

export function useDeviceList() {
  const [devices, setDevices] = useState<BlockDevice[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const refreshDevices = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // In production Tauri environment:
      // const res = await invoke<BlockDevice[]>('scan_devices');
      // setDevices(res);
      setDevices([]);
    } catch (err: unknown) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshDevices();
  }, [refreshDevices]);

  return { devices, loading, error, refreshDevices };
}
