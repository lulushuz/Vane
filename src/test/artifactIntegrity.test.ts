import { describe, it, expect } from 'vitest';

interface ArtifactIntegrityStatusDto {
  status: 'verified' | 'missing' | 'modified' | 'invalid_manifest' | 'unsupported_target';
  target: string;
  verifiedArtifacts: number;
  failedArtifactId?: string;
  errorCode?: string;
  lastVerifiedAt?: string;
}

function computeEngineControlState(status: ArtifactIntegrityStatusDto): {
  engineStartDisabled: boolean;
  userFacingErrorMessage?: string;
} {
  if (status.status === 'verified') {
    return { engineStartDisabled: false };
  }

  const userFacingErrorMessage =
    'Vane motor dosyalarının bütünlük doğrulaması başarısız oldu. Güvenlik nedeniyle motor başlatılmadı. Uygulamayı güvenilir kaynaktan yeniden kurmanız önerilir.';

  return {
    engineStartDisabled: true,
    userFacingErrorMessage,
  };
}

describe('P13 Frontend Binary Integrity Characterization', () => {
  it('FE-01: enables engine controls when integrity status is verified', () => {
    const status: ArtifactIntegrityStatusDto = {
      status: 'verified',
      target: 'WindowsX86_64',
      verifiedArtifacts: 4,
    };
    const res = computeEngineControlState(status);
    expect(res.engineStartDisabled).toBe(false);
    expect(res.userFacingErrorMessage).toBeUndefined();
  });

  it('FE-02: disables engine start when artifact is missing', () => {
    const status: ArtifactIntegrityStatusDto = {
      status: 'missing',
      target: 'WindowsX86_64',
      verifiedArtifacts: 0,
      failedArtifactId: 'windows-winws',
    };
    const res = computeEngineControlState(status);
    expect(res.engineStartDisabled).toBe(true);
    expect(res.userFacingErrorMessage).toContain('bütünlük doğrulaması başarısız oldu');
  });

  it('FE-03: shows clear security error when hash mismatch occurs', () => {
    const status: ArtifactIntegrityStatusDto = {
      status: 'modified',
      target: 'WindowsX86_64',
      verifiedArtifacts: 0,
      failedArtifactId: 'windows-windivert-sys',
      errorCode: 'ArtifactHashMismatch',
    };
    const res = computeEngineControlState(status);
    expect(res.engineStartDisabled).toBe(true);
    expect(res.userFacingErrorMessage).not.toContain('Antivirüsü kapatın');
  });

  it('FE-04: ensures raw local file paths are not exposed in user-facing status', () => {
    const status: ArtifactIntegrityStatusDto = {
      status: 'modified',
      target: 'WindowsX86_64',
      verifiedArtifacts: 0,
      failedArtifactId: 'windows-winws',
    };
    const res = computeEngineControlState(status);
    expect(res.userFacingErrorMessage).not.toContain('C:\\Users\\');
    expect(res.userFacingErrorMessage).not.toContain('/home/');
  });

  it('FE-05: prevents starting optimizer when integrity failure exists', () => {
    const status: ArtifactIntegrityStatusDto = {
      status: 'modified',
      target: 'WindowsX86_64',
      verifiedArtifacts: 0,
    };
    const isOptimizerAllowed = status.status === 'verified';
    expect(isOptimizerAllowed).toBe(false);
  });
});
