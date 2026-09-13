import { useState } from 'react';
import { CloneState } from '../types/clone';

export function useCloneStatus() {
  const [state, setState] = useState<CloneState>('IDLE');
  const [statusMessage, setStatusMessage] = useState<string>('Ready');

  return { state, setState, statusMessage, setStatusMessage };
}
