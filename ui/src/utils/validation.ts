export const REQUIRED_CONFIRMATION_TEXT = 'CLONE TO THIS DISK';

export function isConfirmationValid(input: string): boolean {
  return input.trim() === REQUIRED_CONFIRMATION_TEXT;
}

export function validateCloneCandidate(
  sourceSizeBytes: number,
  destSizeBytes: number,
  isSystemDisk: boolean,
  isMounted: boolean
): { eligible: boolean; reason?: string } {
  if (isSystemDisk) {
    return { eligible: false, reason: 'Destination is the operating system disk' };
  }
  if (isMounted) {
    return { eligible: false, reason: 'Destination has active mounted filesystems' };
  }
  if (destSizeBytes < sourceSizeBytes) {
    return {
      eligible: false,
      reason: `Destination capacity (${destSizeBytes} B) is smaller than source (${sourceSizeBytes} B)`,
    };
  }
  return { eligible: true };
}
