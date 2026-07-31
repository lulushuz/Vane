# P07 — Engine Lifecycle, Process Ownership, Readiness ve Race-Safe State Machine Tamamlama Raporu

**Tarih:** 2026-07-29  
**Sürüm:** 2.1.4  
**Aşama:** P07  

---

## 1. Repository Durumu

- **Branch:** `main`
- **Start Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238`
- **End Commit:** `5e6de56e3dd5d5299f73fa4a4f9ac3732ada9238`
- **Baseline Matched:** Evet (2.1.4)
- **Pre-existing Files:** Korundu

---

## 2. Test Sonuçları

### Frontend
- **Before P07:** 12 test dosyası, 125 test geçti (0 hata)
- **After P07:** 12 test dosyası, 125 test geçti (0 hata)

### Rust Backend
- **Before P07:** 186 test geçti (0 hata, 0 atlanan)
- **After P07:** 193 test geçti (0 hata, 0 atlanan)

---

## 3. Eski ve Yeni Process Ownership Mimarisi

### Old Windows Cleanup
- `lib.rs` içerisinde `taskkill /F /IM winws-x86_64-pc-windows-msvc.exe` çağrılarak executable adıyla eşleşen tüm süreçler öldürülüyordu.
- **Risk:** Kullanıcının çalıştırdığı bağımsız Zapret veya başka WinDivert uygulamaları etkileniyordu.

### New Windows Ownership
- Global `taskkill /IM` tamamen kaldırıldı.
- `PlatformEngineLauncher` standardı ile yalnız Vane tarafından başlatılan child process handle saklanır ve Job Object (`JobObjectGuard`) içerisine eklenir.

### Old Linux Cleanup
- `manager.rs` içerisinde wrapper script `killall nfqws-x86_64-unknown-linux-gnu 2>/dev/null;` komutunu çalıştırıyordu.
- **Risk:** Linux ortamında çalışan tüm nfqws süreçleri öldürülüyordu.

### New Linux Ownership
- Global `killall` launcher script'inden kaldırıldı. Yalnızca Vane'e ait child process PID/process group yönetilir.

---

## 4. Lifecycle State Machine Enum Yapısı

```rust
pub(crate) enum EngineLifecycleState {
    Stopped,
    Preparing {
        operation: EngineOperationId,
        generation: EngineGeneration,
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
    },
    StartingProcess {
        operation: EngineOperationId,
        generation: EngineGeneration,
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
    },
    WaitingForReadiness {
        operation: EngineOperationId,
        generation: EngineGeneration,
        process_id: u32,
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
    },
    RunningUnverified {
        generation: EngineGeneration,
        process_id: u32,
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
    },
    Ready {
        generation: EngineGeneration,
        process_id: u32,
        revision: ConfigRevision,
        fingerprint: ConfigFingerprint,
        readiness: EngineReadiness,
    },
    Restarting {
        operation: EngineOperationId,
        previous_generation: EngineGeneration,
        next_generation: EngineGeneration,
    },
    Stopping {
        operation: EngineOperationId,
        generation: EngineGeneration,
        process_id: u32,
    },
    Failed {
        generation: Option<EngineGeneration>,
        error: LifecycleErrorSummary,
    },
}
```

---

## 5. Async / Race-Condition Çözüm Sonuçları

- **BR-07 (Start then immediate stop):** `engineStore.ts` içerisine eklenen `lifecycleToken` ve backend generation gating sayesinde, yavaş tamamlanan `startEngine` yanıtı `stopEngine` çağrıldıktan sonra store state'ini tekrar `running` yapamaz. (`BR-07 resolved`).
- **RBR-06 (Global Process Cleanup):** `taskkill /IM` ve `killall` kaldırıldı. Sadece Vane tarafından üretilen `OwnedEngineProcess` sonlandırılır (`RBR-06 resolved`).
- **RBR-07 (Running Means PID Alive):** `EngineReadiness` modeli ile PID varlığı ve yerel startup grace süresi ayrıştırıldı (`RBR-07 partially resolved`, trafik sağlığı P14'te ele alınacaktır).

---

## 6. Manuel Acceptance Planı (Windows / Linux)

### Windows Testleri
- **Test 1 (Owned stop):** NOT EXECUTED — requires controlled privileged Windows acceptance environment.
- **Test 2 (Zapret isolation):** NOT EXECUTED — requires controlled privileged Windows acceptance environment.

### Linux Testleri
- **Test 1 (Group kill):** NOT EXECUTED — requires controlled privileged Linux VM environment.

---

## 7. Geçiş Kararı

**READY FOR P08**
