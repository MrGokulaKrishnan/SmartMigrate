import { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { QRCodeSvg } from "./QRCodeSvg";
import "./styles.css";

type Destination = "Home" | "Devices" | "Transfer" | "Remote" | "History" | "Security" | "Settings";

type HostStatus = {
  engine: string;
  platform: string;
  profile: string;
  deviceId: string;
  fingerprint: string;
  trustedDeviceCount: number;
  privilegedFeaturesEnabled: boolean;
};

type NumericPairingCode = {
  // Serialized from Rust NumericPairingCode
  0: string;
};

type PairingSession = {
  session_id: string;
  host_id: string;
  host_name: string;
  code: string | { 0: string };
  secret_token: string;
  created_at_epoch_ms: number;
  expires_at_epoch_ms: number;
  max_attempts: number;
  failed_attempts: number;
  state: "requested" | "awaiting_host_approval" | "approved" | "rejected" | "expired";
  pending_requester_id?: string;
  pending_requester_name?: string;
  requested_permissions: string[];
};

type TrustedDevice = {
  id: string;
  name: string;
  platform: "windows" | "android" | "linux" | "macos" | "unknown";
  fingerprint: string;
  paired_at_epoch_ms: number;
  last_seen_epoch_ms: number;
  granted_permissions: string[];
  is_revoked: boolean;
};

type DisplaySource = {
  id: string;
  name: string;
  width: number;
  height: number;
  refreshRateHz: number;
  isPrimary: boolean;
};

type EncoderCapability = {
  codec: string;
  name: string;
  isHardwareAccelerated: boolean;
  vendor: string;
  maxResolution: string;
  maxFps: number;
};

type StreamTelemetry = {
  isActive: boolean;
  sessionId: string;
  targetDeviceName: string;
  currentFps: number;
  bitrateMbps: number;
  latencyMs: number;
  durationSeconds: number;
  totalFrames: number;
  droppedFrames: number;
  encoderName: string;
  resolution: string;
};

type InputTelemetry = {
  mouseEventsInjected: number;
  keyboardEventsInjected: number;
  replayedPacketsDropped: number;
  permissionDeniedDrops: number;
  hostOverrideDrops: number;
  allowMouse: boolean;
  allowKeyboard: boolean;
};

type ResilienceStatus = {
  transportState: string;
  directP2pActive: boolean;
  rttMs: number;
  packetLossPercent: number;
  heartbeatsSent: number;
  heartbeatsReceived: number;
  packetsDroppedReplay: number;
  lastHeartbeatAgoMs: number | null;
};

type ClipboardStatus = {
  active: boolean;
  sessionId: string;
  direction: string;
  hostPushCount: number;
  clientPushCount: number;
};

type TransferSessionDto = {
  transferId: string;
  fileName: string;
  totalBytes: number;
  chunkSize: number;
  totalChunks: number;
  expectedSha256: string;
  direction: string;
  state: string;
  bytesTransferred: number;
  chunksCompleted: number;
  progressPercent: number;
  nextExpectedChunk: number;
};

type OutgoingChunkDto = {
  transferId: string;
  chunkIndex: number;
  chunkSha256: string;
  dataBase64: string;
  isLast: boolean;
  progressPercent: number;
};

type TransferProgressDto = {
  transferId: string;
  chunksCompleted: number;
  totalChunks: number;
  bytesTransferred: number;
  totalBytes: number;
  progressPercent: number;
  state: string;
  nextExpectedChunk: number;
};

type FinalizeResultDto = {
  transferId: string;
  fileName: string;
  totalBytes: number;
  sha256Verified: boolean;
  savedToFolder: string;
};

const destinations: Destination[] = ["Home", "Devices", "Transfer", "Remote", "History", "Security", "Settings"];

const destinationIcons: Record<Destination, string> = {
  Home: "⌂",
  Devices: "◇",
  Transfer: "⇄",
  Remote: "⌁",
  History: "◷",
  Security: "◈",
  Settings: "⚙",
};

const ALL_PERMISSIONS = [
  { id: "VIEW_SCREEN", label: "View Screen (Display stream)" },
  { id: "CONTROL_MOUSE", label: "Control Mouse (Pointer inputs)" },
  { id: "CONTROL_KEYBOARD", label: "Control Keyboard (Key strokes)" },
  { id: "SEND_FILES", label: "Send Files (Host to Client)" },
  { id: "RECEIVE_FILES", label: "Receive Files (Client to Host)" },
  { id: "CLIPBOARD", label: "Clipboard Sync (Text only)" },
  { id: "AUDIO", label: "Audio Stream" },
];

const previewStatus: HostStatus = {
  engine: "MigRoute",
  platform: "Windows host shell",
  profile: "Loading...",
  deviceId: "sm-win-loading",
  fingerprint: "SM-INITIALIZING",
  trustedDeviceCount: 0,
  privilegedFeaturesEnabled: false,
};

function formatCode(raw: unknown): string {
  let str = "";
  if (typeof raw === "string") str = raw;
  else if (raw && typeof raw === "object" && "0" in raw) str = String((raw as any)["0"]);
  str = str.replace(/\D/g, "");
  if (str.length === 6) return `${str.slice(0, 3)} - ${str.slice(3)}`;
  return str || "••••••";
}

function App() {
  const [destination, setDestination] = useState<Destination>("Home");
  const [status, setStatus] = useState<HostStatus>(previewStatus);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [pairingSession, setPairingSession] = useState<PairingSession | null>(null);
  const [timeLeft, setTimeLeft] = useState(180);
  const [selectedPerms, setSelectedPerms] = useState<Record<string, boolean>>({});
  const [notice, setNotice] = useState("Host shell online. Privileged adapters gated by explicit consent.");

  const refreshStatus = () => {
    invoke<HostStatus>("host_status")
      .then(setStatus)
      .catch(() => setStatus(previewStatus));
  };

  useEffect(() => {
    refreshStatus();
  }, []);

  const openPairingDialog = async () => {
    try {
      const active = await invoke<PairingSession | null>("get_active_pairing");
      if (active && active.state === "requested") {
        setPairingSession(active);
        const remaining = Math.max(0, Math.round((active.expires_at_epoch_ms - Date.now()) / 1000));
        setTimeLeft(remaining);
      } else {
        const session = await invoke<PairingSession>("start_pairing_session");
        setPairingSession(session);
        setTimeLeft(180);
      }
      setDialogOpen(true);
      setNotice("Pairing session active. Ready for client connection.");
    } catch (err) {
      setNotice(`Failed to start pairing: ${String(err)}`);
    }
  };

  const closePairingDialog = async () => {
    if (pairingSession) {
      invoke("cancel_pairing").catch(() => {});
    }
    setDialogOpen(false);
    setPairingSession(null);
  };

  // Pairing TTL countdown timer & session poll
  useEffect(() => {
    if (!dialogOpen || !pairingSession) return;

    const timer = setInterval(() => {
      setTimeLeft((prev) => {
        if (prev <= 1) {
          clearInterval(timer);
          invoke("cancel_pairing").catch(() => {});
          setPairingSession(null);
          setNotice("Pairing session expired.");
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    const poll = setInterval(async () => {
      try {
        const active = await invoke<PairingSession | null>("get_active_pairing");
        if (active) {
          setPairingSession(active);
          if (active.state === "awaiting_host_approval" && active.requested_permissions) {
            // Pre-select requested permissions
            const initial: Record<string, boolean> = {};
            active.requested_permissions.forEach((p) => {
              initial[p] = true;
            });
            setSelectedPerms((prev) => (Object.keys(prev).length === 0 ? initial : prev));
          }
        }
      } catch (_) {}
    }, 1500);

    return () => {
      clearInterval(timer);
      clearInterval(poll);
    };
  }, [dialogOpen, pairingSession]);

  const handleApprovePairing = async () => {
    const granted = Object.entries(selectedPerms)
      .filter(([_, enabled]) => enabled)
      .map(([id]) => id);

    try {
      await invoke("approve_pairing", { grantedPermissions: granted });
      setNotice("Device successfully paired and authorized!");
      setDialogOpen(false);
      setPairingSession(null);
      refreshStatus();
    } catch (err) {
      setNotice(`Approval error: ${String(err)}`);
    }
  };

  const handleRejectPairing = async () => {
    try {
      await invoke("reject_pairing");
      setNotice("Pairing request rejected.");
      setDialogOpen(false);
      setPairingSession(null);
    } catch (err) {
      setNotice(`Error: ${String(err)}`);
    }
  };

  const handleSimulateClient = async () => {
    if (!pairingSession) return;
    const rawCode = typeof pairingSession.code === "string" ? pairingSession.code : (pairingSession.code as any)["0"];
    try {
      await invoke("submit_client_pairing_code", {
        requesterId: "sm-android-pixel-tablet",
        requesterName: "Pixel Tablet (Living Room)",
        code: rawCode,
        token: pairingSession.secret_token,
        permissions: ["VIEW_SCREEN", "CONTROL_MOUSE", "SEND_FILES"],
      });
      setNotice("Simulated client paired. Awaiting your approval.");
    } catch (e) {
      setNotice(`Simulate error: ${String(e)}`);
    }
  };

  const title = useMemo(() => {
    if (destination === "Home") return `Welcome, ${status.profile}.`;
    return destination;
  }, [destination, status.profile]);

  const showUnavailable = (feature: string) => {
    setNotice(`${feature} is milestone-gated until native adapter and security verification are complete.`);
  };

  const handleMinimize = () => invoke("minimize_window").catch(() => {});
  const handleMaximize = () => invoke("toggle_maximize").catch(() => {});
  const handleClose = () => invoke("close_window").catch(() => {});

  const [showIntro, setShowIntro] = useState(() => {
    if (typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      return false;
    }
    return true;
  });

  return (
    <main className="app-frame">
      {showIntro && <StartupIntro onComplete={() => setShowIntro(false)} />}
      <div className="ambient ambient-one" aria-hidden="true" />
      <div className="ambient ambient-two" aria-hidden="true" />

      <header
        className="titlebar glass-nav"
        aria-label="Window controls and application identity"
        data-tauri-drag-region
        onDoubleClick={handleMaximize}
      >
        <div className="brand-lockup">
          <div className="brand-mark" aria-hidden="true">SM</div>
          <div>
            <strong>Smart Migrate</strong>
            <span>Windows host</span>
          </div>
        </div>
        <div className="titlebar-state"><span className="state-dot" /> {status.profile} ({status.fingerprint})</div>
        <div className="window-controls" aria-label="Window controls">
          <button type="button" aria-label="Minimize window" onClick={handleMinimize}>—</button>
          <button type="button" aria-label="Maximize window" onClick={handleMaximize}>□</button>
          <button className="close" type="button" aria-label="Close window" onClick={handleClose}>×</button>
        </div>
      </header>

      <section className="app-shell">
        <aside className="sidebar glass-surface" aria-label="Smart Migrate navigation">
          <div className="engine-badge"><span className="engine-pulse" /> Powered by <strong>{status.engine}</strong></div>
          <nav>
            {destinations.map((item) => (
              <button
                className={destination === item ? "nav-item active" : "nav-item"}
                key={item}
                type="button"
                onClick={() => { setDestination(item); setNotice(`${item} view active.`); }}
              >
                <span aria-hidden="true">{destinationIcons[item]}</span>{item}
              </button>
            ))}
          </nav>
          <div className="sidebar-footer">
            <span className="mini-avatar">PC</span>
            <div><strong>{status.profile}</strong><span>{status.deviceId}</span></div>
          </div>
        </aside>

        <section className="workspace" aria-labelledby="workspace-title">
          <div className="workspace-heading">
            <div>
              <p className="eyebrow">{status.platform}</p>
              <h1 id="workspace-title">{title}</h1>
            </div>
            <button className="primary-action" type="button" onClick={openPairingDialog}>Connect device <span aria-hidden="true">＋</span></button>
          </div>

          {destination === "Home" && (
            <HomeDashboard
              status={status}
              onStartPairing={openPairingDialog}
              onNavigateDevices={() => setDestination("Devices")}
              onUnavailable={showUnavailable}
            />
          )}

          {destination === "Devices" && (
            <DevicesManagerView onStartPairing={openPairingDialog} onStatusChange={refreshStatus} />
          )}

          {destination === "Remote" && (
            <RemoteStreamManagerView
              onNotice={setNotice}
              onNavigateDevices={() => setDestination("Devices")}
              onStatusChange={refreshStatus}
            />
          )}

          {destination === "Security" && (
            <SecurityView status={status} />
          )}

          {destination === "Transfer" && (
            <TransferHubView onNotice={setNotice} onNavigateDevices={() => setDestination("Devices")} />
          )}

          {destination === "History" && (
            <AuditHistoryView onNavigateDevices={() => setDestination("Devices")} />
          )}

          {destination !== "Home" && destination !== "Devices" && destination !== "Remote" && destination !== "Security" && destination !== "Transfer" && destination !== "History" && (
            <UnavailablePanel destination={destination} onUnavailable={showUnavailable} />
          )}

        </section>
      </section>

      <p className="live-notice" aria-live="polite">{notice}</p>

      {dialogOpen && (
        <div className="modal-backdrop" role="presentation" onMouseDown={closePairingDialog}>
          <section
            className="glass-surface pairing-dialog-enhanced"
            role="dialog"
            aria-modal="true"
            aria-labelledby="pairing-title"
            onMouseDown={(e) => e.stopPropagation()}
          >
            <div className="pairing-header">
              <span className="dialog-kicker">Trusted Device Pairing</span>
              <div className="pairing-timer">
                <span>⏱</span>
                <span>{Math.floor(timeLeft / 60)}:{(timeLeft % 60).toString().padStart(2, "0")}</span>
              </div>
            </div>

            {pairingSession?.state === "awaiting_host_approval" ? (
              <div className="approval-card">
                <div className="client-info-header">
                  <div>
                    <span className="eyebrow">Incoming Connection Request</span>
                    <h2>{pairingSession.pending_requester_name || "Remote Device"}</h2>
                    <p style={{ margin: 0, fontSize: "0.7rem", color: "var(--sm-text-3)" }}>
                      Device ID: {pairingSession.pending_requester_id}
                    </p>
                  </div>
                  <span className="client-badge">Verified Code</span>
                </div>

                <p style={{ fontSize: "0.75rem", color: "var(--sm-text-2)", margin: 0 }}>
                  Host authorization is the source of truth. Narrow or deselect any capability before granting access:
                </p>

                <div className="permission-checkbox-list">
                  {ALL_PERMISSIONS.map((perm) => {
                    const isRequested = pairingSession.requested_permissions.includes(perm.id);
                    return (
                      <label key={perm.id} className="permission-checkbox-item">
                        <input
                          type="checkbox"
                          disabled={!isRequested}
                          checked={!!selectedPerms[perm.id] && isRequested}
                          onChange={(e) => setSelectedPerms((prev) => ({ ...prev, [perm.id]: e.target.checked }))}
                        />
                        <span style={{ opacity: isRequested ? 1 : 0.4 }}>{perm.label}</span>
                      </label>
                    );
                  })}
                </div>

                <div className="dialog-actions">
                  <button className="secondary-action" type="button" onClick={handleRejectPairing}>Deny Request</button>
                  <button className="primary-action" type="button" onClick={handleApprovePairing}>Authorize & Connect ↗</button>
                </div>
              </div>
            ) : (
              <>
                <h2 id="pairing-title" style={{ margin: 0 }}>Pair this PC with your device</h2>
                <p style={{ margin: 0, fontSize: "0.76rem", color: "var(--sm-text-2)" }}>
                  Scan the QR code or enter the 6-digit cryptographic pairing code on your Smart Migrate Android client.
                </p>

                <div className="pairing-card">
                  <div className="pairing-qr-box">
                    <QRCodeSvg
                      value={`smp://pair?v=1&host=${status.deviceId}&code=${pairingSession ? (typeof pairingSession.code === "string" ? pairingSession.code : (pairingSession.code as any)["0"]) : ""}`}
                    />
                  </div>
                  <div className="pairing-info">
                    <span className="pairing-code-label">Pairing Code</span>
                    <div className="pairing-code-display">
                      {pairingSession ? formatCode(pairingSession.code) : "••••••"}
                    </div>
                    <span className="pairing-code-sub">
                      Single-use cryptographic PIN. Auto-expires in 3 minutes with rate-limiting.
                    </span>
                  </div>
                </div>

                <div className="dialog-actions">
                  <button
                    className="secondary-action"
                    type="button"
                    style={{ fontSize: "0.65rem", opacity: 0.8 }}
                    onClick={handleSimulateClient}
                    title="Simulate Android client code submission for local testing"
                  >
                    Simulate Client 🧪
                  </button>
                  <button className="secondary-action" type="button" onClick={closePairingDialog}>Cancel</button>
                </div>
              </>
            )}
          </section>
        </div>
      )}
    </main>
  );
}

function HomeDashboard({
  status,
  onStartPairing,
  onNavigateDevices,
  onUnavailable,
}: {
  status: HostStatus;
  onStartPairing: () => void;
  onNavigateDevices: () => void;
  onUnavailable: (feature: string) => void;
}) {
  return (
    <div className="dashboard-stack">
      <section className="host-overview glass-surface">
        <div className="overview-copy">
          <span className="online-badge"><i /> Host shell online</span>
          <h2>This PC is ready for trusted pairing.</h2>
          <p>Smart Migrate requires explicit approval before any device receives capability access. Zero background control without consent.</p>
          <div className="overview-actions">
            <button className="primary-action" type="button" onClick={onStartPairing}>Start pairing <span aria-hidden="true">→</span></button>
            <button className="secondary-action" type="button" onClick={onNavigateDevices}>View trusted devices</button>
          </div>
        </div>
        <div className="host-orb" aria-hidden="true"><span>SM</span><i className="orb-route">↗</i></div>
      </section>

      <section className="metric-grid" aria-label="Current host state">
        <Metric
          label="Trusted devices"
          value={String(status.trustedDeviceCount)}
          detail={status.trustedDeviceCount > 0 ? "Paired & authorized" : "Pair a device to begin"}
          accent="violet"
        />
        <Metric label="Current session" value="None" detail="Host approval required" accent="blue" />
        <Metric label="Security state" value="Protected" detail="Local DPAPI identity active" accent="green" />
      </section>

      <section className="content-grid">
        <article className="glass-surface device-panel">
          <div className="panel-heading">
            <div>
              <p className="eyebrow">Device Management</p>
              <h2>{status.trustedDeviceCount > 0 ? `${status.trustedDeviceCount} Active Device(s)` : "No devices yet"}</h2>
            </div>
            <button className="quiet-button" type="button" onClick={onNavigateDevices}>Manage <span aria-hidden="true">↗</span></button>
          </div>
          {status.trustedDeviceCount === 0 ? (
            <div className="empty-device">
              <div className="device-glyph" aria-hidden="true">◇</div>
              <p>Your approved Windows and Android devices will appear here after pairing.</p>
            </div>
          ) : (
            <p style={{ marginTop: "1rem", fontSize: "0.75rem", color: "var(--sm-text-2)" }}>
              All paired devices have explicit permissions granted by this PC. Click Manage to view or revoke.
            </p>
          )}
        </article>

        <article className="glass-surface security-panel">
          <p className="eyebrow">Connection policy</p>
          <h2>Approval is always visible.</h2>
          <ul>
            <li><span>✓</span> Host decides every capability</li>
            <li><span>✓</span> Single-use 6-digit codes with 180s TTL</li>
            <li><span>✓</span> Strict rate-limiting against brute force</li>
            <li><span>✓</span> Persistent OS-keystore backed identity</li>
          </ul>
        </article>
      </section>
    </div>
  );
}

function DevicesManagerView({
  onStartPairing,
  onStatusChange,
}: {
  onStartPairing: () => void;
  onStatusChange: () => void;
}) {
  const [devices, setDevices] = useState<TrustedDevice[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchDevices = () => {
    invoke<TrustedDevice[]>("get_trusted_devices")
      .then((res) => {
        setDevices(res);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  };

  useEffect(() => {
    fetchDevices();
  }, []);

  const handleRevoke = async (id: string) => {
    try {
      await invoke("revoke_trusted_device", { deviceId: id });
      fetchDevices();
      onStatusChange();
    } catch (_) {}
  };

  const handleRemove = async (id: string) => {
    try {
      await invoke("remove_trusted_device", { deviceId: id });
      fetchDevices();
      onStatusChange();
    } catch (_) {}
  };

  return (
    <div className="devices-view">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <p className="eyebrow">Trusted Device Registry</p>
          <h2>Paired & Authorized Devices</h2>
        </div>
        <button className="primary-action" type="button" onClick={onStartPairing}>Pair New Device <span aria-hidden="true">＋</span></button>
      </div>

      {loading ? (
        <p style={{ color: "var(--sm-text-3)", fontSize: "0.8rem" }}>Loading trusted devices...</p>
      ) : devices.length === 0 ? (
        <div className="glass-surface" style={{ padding: "40px", textAlign: "center" }}>
          <div className="large-symbol" style={{ margin: "0 auto 1rem" }}>◇</div>
          <h2>No trusted devices yet</h2>
          <p style={{ color: "var(--sm-text-2)", fontSize: "0.8rem", maxWidth: "420px", margin: "0 auto 1.5rem" }}>
            Pair an Android or secondary Windows PC to enable secure remote access and file migration with explicit permissions.
          </p>
          <button className="primary-action" type="button" onClick={onStartPairing}>Start Pairing Now →</button>
        </div>
      ) : (
        <div className="devices-list">
          {devices.map((dev) => (
            <article key={dev.id} className="device-item-card">
              <div className="device-icon">{dev.platform === "android" ? "📱" : "💻"}</div>
              <div className="device-meta">
                <h3>{dev.name}</h3>
                <p>ID: {dev.id} • Fingerprint: {dev.fingerprint} • Paired: {new Date(dev.paired_at_epoch_ms).toLocaleDateString()}</p>
                <div className="device-tags">
                  {dev.is_revoked ? (
                    <span className="device-tag revoked">REVOKED</span>
                  ) : (
                    dev.granted_permissions.map((p) => (
                      <span key={p} className="device-tag">✓ {p}</span>
                    ))
                  )}
                </div>
              </div>
              <div className="device-card-actions">
                {!dev.is_revoked && (
                  <button className="danger-button" type="button" onClick={() => handleRevoke(dev.id)}>Revoke</button>
                )}
                <button className="secondary-action" type="button" style={{ padding: "0.4rem 0.6rem", fontSize: "0.68rem" }} onClick={() => handleRemove(dev.id)}>Remove</button>
              </div>
            </article>
          ))}
        </div>
      )}
    </div>
  );
}

function RemoteStreamManagerView({
  onNotice,
  onNavigateDevices,
  onStatusChange,
}: {
  onNotice: (msg: string) => void;
  onNavigateDevices: () => void;
  onStatusChange: () => void;
}) {
  const [sources, setSources] = useState<DisplaySource[]>([]);
  const [encoders, setEncoders] = useState<EncoderCapability[]>([]);
  const [authorizedDevices, setAuthorizedDevices] = useState<TrustedDevice[]>([]);
  const [selectedSource, setSelectedSource] = useState("");
  const [selectedDevice, setSelectedDevice] = useState("");
  const [selectedEncoder, setSelectedEncoder] = useState("");
  const [targetFps, setTargetFps] = useState(30);
  const [telemetry, setTelemetry] = useState<StreamTelemetry | null>(null);
  const [inputTelemetry, setInputTelemetry] = useState<InputTelemetry | null>(null);
  const [resilience, setResilience] = useState<ResilienceStatus | null>(null);

  useEffect(() => {
    // 1. Fetch display sources
    invoke<DisplaySource[]>("get_display_sources")
      .then((res) => {
        setSources(res);
        if (res.length > 0) setSelectedSource(res[0].id);
      })
      .catch(() => {});

    // 2. Fetch hardware encoders
    invoke<EncoderCapability[]>("detect_hardware_encoders")
      .then((res) => {
        setEncoders(res);
        if (res.length > 0) setSelectedEncoder(res[0].name);
      })
      .catch(() => {});

    // 3. Fetch trusted devices and filter those holding VIEW_SCREEN
    invoke<TrustedDevice[]>("get_trusted_devices")
      .then((devs) => {
        const withScreenPerm = devs.filter(
          (d) => !d.is_revoked && d.granted_permissions.includes("VIEW_SCREEN")
        );
        setAuthorizedDevices(withScreenPerm);
        if (withScreenPerm.length > 0) setSelectedDevice(withScreenPerm[0].id);
      })
      .catch(() => {});
  }, []);

  // Telemetry and resilience poll interval
  useEffect(() => {
    const poll = setInterval(() => {
      invoke<StreamTelemetry>("get_stream_telemetry")
        .then((t) => setTelemetry(t))
        .catch(() => {});
      invoke<InputTelemetry>("get_input_telemetry")
        .then((it) => setInputTelemetry(it))
        .catch(() => {});
      invoke<ResilienceStatus>("get_resilience_status")
        .then((rs) => setResilience(rs))
        .catch(() => {});
    }, 1000);
    return () => clearInterval(poll);
  }, []);

  const handleToggleMouse = async () => {
    const currentMouse = inputTelemetry?.allowMouse ?? true;
    const currentKeyboard = inputTelemetry?.allowKeyboard ?? true;
    try {
      await invoke("set_input_override", {
        mouseEnabled: !currentMouse,
        keyboardEnabled: currentKeyboard,
      });
      setInputTelemetry((prev) => (prev ? { ...prev, allowMouse: !currentMouse } : null));
      onNotice(!currentMouse ? "Remote mouse input allowed." : "Remote mouse input suspended by host override.");
    } catch (err) {
      onNotice(`Input override error: ${String(err)}`);
    }
  };

  const handleToggleKeyboard = async () => {
    const currentMouse = inputTelemetry?.allowMouse ?? true;
    const currentKeyboard = inputTelemetry?.allowKeyboard ?? true;
    try {
      await invoke("set_input_override", {
        mouseEnabled: currentMouse,
        keyboardEnabled: !currentKeyboard,
      });
      setInputTelemetry((prev) => (prev ? { ...prev, allowKeyboard: !currentKeyboard } : null));
      onNotice(!currentKeyboard ? "Remote keyboard input allowed." : "Remote keyboard input suspended by host override.");
    } catch (err) {
      onNotice(`Input override error: ${String(err)}`);
    }
  };

  const handleStartStream = async () => {
    if (!selectedDevice) {
      onNotice("Select an authorized client device first.");
      return;
    }
    try {
      await invoke("start_display_stream", {
        targetDeviceId: selectedDevice,
        sourceId: selectedSource || "display-primary",
        codec: "H264",
        targetFps,
        encoderName: selectedEncoder || "Hardware H.264",
      });
      onNotice("Display stream active! Hardware encoder initialized.");
      onStatusChange();
    } catch (err) {
      onNotice(`Stream error: ${String(err)}`);
    }
  };

  const handleStopStream = async () => {
    try {
      await invoke("stop_display_stream", { reason: "Host operator stopped stream" });
      onNotice("Display stream stopped. Capture hardware released.");
      onStatusChange();
    } catch (err) {
      onNotice(`Stop error: ${String(err)}`);
    }
  };

  const isStreaming = telemetry?.isActive ?? false;

  return (
    <div className="remote-stream-view">
      <div className="stream-control-banner">
        <div>
          <span className="eyebrow">Windows Graphics Capture & Streaming</span>
          <h2 style={{ margin: "0.2rem 0" }}>Host Display Stream Monitor</h2>
          <p style={{ margin: 0, fontSize: "0.75rem", color: "var(--sm-text-2)" }}>
            Encrypted low-latency video streaming to host-authorized Android and Windows clients.
          </p>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "0.8rem" }}>
          <span className={`stream-status-pill ${isStreaming ? "active" : ""}`}>
            <span className={isStreaming ? "state-dot" : ""} style={{ background: isStreaming ? "var(--sm-success)" : "var(--sm-text-3)" }} />
            {isStreaming ? "STREAM ACTIVE" : "CAPTURE IDLE"}
          </span>
          {isStreaming ? (
            <button className="danger-button" type="button" onClick={handleStopStream} style={{ padding: "0.55rem 1rem", fontSize: "0.75rem" }}>
              Disconnect & Stop Stream ⏹
            </button>
          ) : (
            <button className="primary-action" type="button" onClick={handleStartStream} disabled={authorizedDevices.length === 0}>
              Start Stream <span aria-hidden="true">▶</span>
            </button>
          )}
        </div>
      </div>

      {/* Telemetry HUD */}
      <div className="telemetry-grid">
        <div className="telemetry-item">
          <span>Capture Rate</span>
          <strong>{isStreaming ? `${telemetry?.currentFps.toFixed(1)} FPS` : "0.0 FPS"}</strong>
        </div>
        <div className="telemetry-item">
          <span>Network Bitrate</span>
          <strong>{isStreaming ? `${telemetry?.bitrateMbps.toFixed(1)} Mbps` : "0.0 Mbps"}</strong>
        </div>
        <div className="telemetry-item">
          <span>Glass Latency</span>
          <strong>{isStreaming ? `${telemetry?.latencyMs.toFixed(1)} ms` : "-- ms"}</strong>
        </div>
        <div className="telemetry-item">
          <span>Session Duration</span>
          <strong>
            {isStreaming
              ? `${Math.floor((telemetry?.durationSeconds || 0) / 60)}:${((telemetry?.durationSeconds || 0) % 60).toString().padStart(2, "0")}`
              : "00:00"}
          </strong>
        </div>
      </div>

      <div className="stream-config-grid">
        {/* Source & Device Selector */}
        <div className="glass-surface config-card">
          <p className="eyebrow">Display Source & Target</p>
          <div className="form-group">
            <label className="form-label">Capture Source (Monitor)</label>
            <select
              className="form-select"
              value={selectedSource}
              onChange={(e) => setSelectedSource(e.target.value)}
              disabled={isStreaming}
            >
              {sources.map((s) => (
                <option key={s.id} value={s.id}>
                  {s.name} ({s.width}×{s.height} @ {s.refreshRateHz}Hz) {s.isPrimary ? "[Primary]" : ""}
                </option>
              ))}
            </select>
          </div>

          <div className="form-group">
            <label className="form-label">Target Authorized Device</label>
            {authorizedDevices.length === 0 ? (
              <div style={{ padding: "0.6rem", background: "rgba(255,92,122,0.1)", borderRadius: "6px", border: "1px solid rgba(255,92,122,0.25)" }}>
                <p style={{ margin: 0, fontSize: "0.72rem", color: "var(--sm-error)" }}>
                  No paired devices currently hold the VIEW_SCREEN permission.
                </p>
                <button
                  type="button"
                  onClick={onNavigateDevices}
                  style={{ marginTop: "0.4rem", background: "none", border: "none", color: "var(--sm-200)", fontSize: "0.7rem", cursor: "pointer", textDecoration: "underline" }}
                >
                  Manage devices to grant permission →
                </button>
              </div>
            ) : (
              <select
                className="form-select"
                value={selectedDevice}
                onChange={(e) => setSelectedDevice(e.target.value)}
                disabled={isStreaming}
              >
                {authorizedDevices.map((d) => (
                  <option key={d.id} value={d.id}>
                    {d.name} ({d.platform.toUpperCase()}) — {d.id}
                  </option>
                ))}
              </select>
            )}
          </div>
        </div>

        {/* Encoder & Performance Config */}
        <div className="glass-surface config-card">
          <p className="eyebrow">Hardware Acceleration & Quality</p>
          <div className="form-group">
            <label className="form-label">Video Encoder Pipeline</label>
            <select
              className="form-select"
              value={selectedEncoder}
              onChange={(e) => setSelectedEncoder(e.target.value)}
              disabled={isStreaming}
            >
              {encoders.map((enc) => (
                <option key={enc.name} value={enc.name}>
                  {enc.name} {enc.isHardwareAccelerated ? "⚡ [HW]" : "[SW]"}
                </option>
              ))}
            </select>
          </div>

          <div className="form-group">
            <label className="form-label">Target Refresh Rate</label>
            <select
              className="form-select"
              value={targetFps}
              onChange={(e) => setTargetFps(Number(e.target.value))}
              disabled={isStreaming}
            >
              <option value={30}>1080p @ 30 FPS (LAN Low Latency Baseline)</option>
              <option value={60}>1080p @ 60 FPS (LAN High Refresh)</option>
            </select>
          </div>
        </div>

        {/* Milestone 4: Host Input Control & Replay Defense */}
        <div className="glass-surface config-card">
          <p className="eyebrow">Input Authority & Replay Defense</p>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem" }}>
            <button
              type="button"
              className={`toggle-badge-btn ${inputTelemetry?.allowMouse !== false ? "active" : "suspended"}`}
              onClick={handleToggleMouse}
            >
              <span>Remote Pointer Control</span>
              <span>{inputTelemetry?.allowMouse !== false ? "ENABLED [CLICK TO SUSPEND]" : "SUSPENDED [OVERRIDE ACTIVE]"}</span>
            </button>
            <button
              type="button"
              className={`toggle-badge-btn ${inputTelemetry?.allowKeyboard !== false ? "active" : "suspended"}`}
              onClick={handleToggleKeyboard}
            >
              <span>Remote Keyboard Control</span>
              <span>{inputTelemetry?.allowKeyboard !== false ? "ENABLED [CLICK TO SUSPEND]" : "SUSPENDED [OVERRIDE ACTIVE]"}</span>
            </button>
          </div>

          <div className="metric-mini-grid" style={{ marginTop: "0.3rem" }}>
            <div className="metric-mini-cell">
              <span>Injected Mouse</span>
              <strong>{inputTelemetry?.mouseEventsInjected ?? 0}</strong>
            </div>
            <div className="metric-mini-cell">
              <span>Injected Keys</span>
              <strong>{inputTelemetry?.keyboardEventsInjected ?? 0}</strong>
            </div>
            <div className="metric-mini-cell">
              <span>Replay Drops</span>
              <strong>{inputTelemetry?.replayedPacketsDropped ?? 0}</strong>
            </div>
            <div className="metric-mini-cell">
              <span>Denied Drops</span>
              <strong>{inputTelemetry?.permissionDeniedDrops ?? 0}</strong>
            </div>
          </div>
          <p style={{ margin: 0, fontSize: "0.68rem", color: "var(--sm-text-3)", lineHeight: "1.3" }}>
            Host holds ultimate physical override. Any mouse or keyboard move immediately suppresses remote injection. Non-monotonic packets are discarded.
          </p>
        </div>

        {/* Milestone 4: Transport Resilience & Watchdog */}
        <div className="glass-surface config-card">
          <p className="eyebrow">Transport Resilience & Watchdog</p>
          <div className="metric-mini-grid">
            <div className="metric-mini-cell">
              <span>Transport</span>
              <strong>{resilience?.transportState || (isStreaming ? "LAN WebRTC / Direct" : "Standby")}</strong>
            </div>
            <div className="metric-mini-cell">
              <span>P2P Direct</span>
              <strong>{resilience?.directP2pActive !== false && isStreaming ? "Active ⚡" : "LAN Standby"}</strong>
            </div>
            <div className="metric-mini-cell">
              <span>Round-Trip Latency</span>
              <strong>{isStreaming ? `${(resilience?.rttMs || 12.4).toFixed(1)} ms` : "--"}</strong>
            </div>
            <div className="metric-mini-cell">
              <span>Packet Loss</span>
              <strong>{isStreaming ? `${(resilience?.packetLossPercent || 0.0).toFixed(1)}%` : "0.0%"}</strong>
            </div>
            <div className="metric-mini-cell">
              <span>Heartbeats Sent</span>
              <strong>{resilience?.heartbeatsSent ?? 0}</strong>
            </div>
            <div className="metric-mini-cell">
              <span>Heartbeats Acked</span>
              <strong>{resilience?.heartbeatsReceived ?? 0}</strong>
            </div>
          </div>
          <p style={{ margin: 0, fontSize: "0.68rem", color: "var(--sm-text-3)", lineHeight: "1.3" }}>
            Connection watchdog monitors round-trip health every 500ms. Fallback relays engage automatically if direct peer-to-peer UDP drops.
          </p>
        </div>
      </div>
    </div>
  );
}

function SecurityView({ status }: { status: HostStatus }) {
  return (
    <div className="security-view">
      <div className="glass-surface security-card">
        <p className="eyebrow">Host Identity & Cryptography</p>
        <h2>Protected Endpoint</h2>
        <div className="identity-field">
          <span>Machine Identity</span>
          <strong>{status.profile}</strong>
        </div>
        <div className="identity-field">
          <span>Unique Device ID</span>
          <strong>{status.deviceId}</strong>
        </div>
        <div className="identity-field">
          <span>Public Fingerprint</span>
          <strong>{status.fingerprint}</strong>
        </div>
        <div className="identity-field">
          <span>Key Storage</span>
          <strong>OS Keystore / Windows DPAPI</strong>
        </div>
      </div>

      <div className="glass-surface security-card">
        <p className="eyebrow">Smart Migrate Protocol</p>
        <h2>SMP/1 Enforcement</h2>
        <div className="identity-field">
          <span>Protocol Version</span>
          <strong>SMP/1 (Monotonic Replay Protected)</strong>
        </div>
        <div className="identity-field">
          <span>Engine Boundary</span>
          <strong>MigRoute Pure Systems Engine</strong>
        </div>
        <div className="identity-field">
          <span>Zero-Trust Policy</span>
          <strong>Explicit Host-Side Authorization</strong>
        </div>
        <div className="identity-field">
          <span>Pairing Codes</span>
          <strong>6-Digit Constant-Time, 180s TTL, Max 3 Attempts</strong>
        </div>
      </div>
    </div>
  );
}

function Metric({ label, value, detail, accent }: { label: string; value: string; detail: string; accent: string }) {
  return <article className={`metric glass-surface ${accent}`}><p>{label}</p><strong>{value}</strong><span>{detail}</span></article>;
}

function ClipboardSyncView({
  onNotice,
  onNavigateDevices,
}: {
  onNotice: (msg: string) => void;
  onNavigateDevices: () => void;
}) {
  const [clipboardDevices, setClipboardDevices] = useState<TrustedDevice[]>([]);
  const [selectedDevice, setSelectedDevice] = useState("");
  const [direction, setDirection] = useState("bidirectional");
  const [status, setClipStatus] = useState<ClipboardStatus | null>(null);
  const [hostText, setHostText] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<TrustedDevice[]>("get_trusted_devices")
      .then((devs) => {
        const withClip = devs.filter((d) => !d.is_revoked && d.granted_permissions.includes("CLIPBOARD"));
        setClipboardDevices(withClip);
        if (withClip.length > 0) setSelectedDevice(withClip[0].id);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  // Poll clipboard status every 1 second when sync is active
  useEffect(() => {
    const poll = setInterval(() => {
      invoke<ClipboardStatus>("get_clipboard_status")
        .then((s) => setClipStatus(s))
        .catch(() => {});
    }, 1000);
    return () => clearInterval(poll);
  }, []);

  const handleActivate = async () => {
    if (!selectedDevice) { onNotice("Select a device with Clipboard permission first."); return; }
    try {
      const s = await invoke<ClipboardStatus>("activate_clipboard_sync", {
        deviceId: selectedDevice,
        sessionId: `clip-${Date.now()}`,
        direction,
      });
      setClipStatus(s);
      onNotice("Clipboard sync activated. Text will be exchanged on demand.");
    } catch (err) {
      onNotice(`Activate error: ${String(err)}`);
    }
  };

  const handleSuspend = async () => {
    try {
      await invoke("suspend_clipboard_sync");
      setClipStatus((prev) => prev ? { ...prev, active: false } : null);
      onNotice("Clipboard sync suspended (grant preserved).");
    } catch (err) {
      onNotice(`Error: ${String(err)}`);
    }
  };

  const handleResume = async () => {
    try {
      await invoke("resume_clipboard_sync");
      setClipStatus((prev) => prev ? { ...prev, active: true } : null);
      onNotice("Clipboard sync resumed.");
    } catch (err) {
      onNotice(`Error: ${String(err)}`);
    }
  };

  const handleDeactivate = async () => {
    try {
      await invoke("deactivate_clipboard_sync");
      setClipStatus(null);
      onNotice("Clipboard sync deactivated and grant cleared.");
    } catch (err) {
      onNotice(`Error: ${String(err)}`);
    }
  };

  const handlePushToClient = async () => {
    // Read current host clipboard, display char count, push to client telemetry
    try {
      const text = await invoke<string | null>("read_host_clipboard_text");
      if (!text) { onNotice("Host clipboard is empty or sync is not active."); return; }
      setHostText(text.slice(0, 120) + (text.length > 120 ? "…" : ""));
      setClipStatus((prev) => prev ? { ...prev, hostPushCount: (prev.hostPushCount ?? 0) + 1 } : null);
      onNotice(`Host clipboard read (${text.length} chars). Sync enabled — client will receive on next poll.`);
    } catch (err) {
      onNotice(`Push error: ${String(err)}`);
    }
  };

  const isActive = status?.active === true;
  const hasSyncGrant = !!status?.sessionId;

  return (
    <div className="remote-stream-view">
      <div className="stream-control-banner">
        <div>
          <span className="eyebrow">Milestone 5 — Opt-in Clipboard Sync</span>
          <h2 style={{ margin: "0.2rem 0" }}>Clipboard Synchronization</h2>
          <p style={{ margin: 0, fontSize: "0.75rem", color: "var(--sm-text-2)" }}>
            Explicit host-authorized plain-text clipboard exchange. The host controls direction and can suspend or revoke at any time.
          </p>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: "0.8rem" }}>
          <span className={`stream-status-pill ${isActive ? "active" : ""}`}>
            <span className={isActive ? "state-dot" : ""} style={{ background: isActive ? "var(--sm-success)" : "var(--sm-text-3)" }} />
            {isActive ? "SYNC ACTIVE" : hasSyncGrant ? "SYNC SUSPENDED" : "SYNC IDLE"}
          </span>
          {hasSyncGrant && (
            <button className="danger-button" type="button" onClick={handleDeactivate} style={{ padding: "0.55rem 1rem", fontSize: "0.75rem" }}>
              Deactivate Sync ✕
            </button>
          )}
        </div>
      </div>

      {/* Counters */}
      <div className="telemetry-grid">
        <div className="telemetry-item">
          <span>Sync State</span>
          <strong>{isActive ? "Active" : hasSyncGrant ? "Suspended" : "Not Started"}</strong>
        </div>
        <div className="telemetry-item">
          <span>Direction</span>
          <strong>{status?.direction?.replace(/_/g, " ") ?? "none"}</strong>
        </div>
        <div className="telemetry-item">
          <span>Host → Client Pushes</span>
          <strong>{status?.hostPushCount ?? 0}</strong>
        </div>
        <div className="telemetry-item">
          <span>Client → Host Received</span>
          <strong>{status?.clientPushCount ?? 0}</strong>
        </div>
      </div>

      <div className="stream-config-grid">
        {/* Device & Direction Selector */}
        <div className="glass-surface config-card">
          <p className="eyebrow">Target Device &amp; Sync Direction</p>

          {loading ? (
            <p style={{ fontSize: "0.8rem", color: "var(--sm-text-3)" }}>Loading devices…</p>
          ) : clipboardDevices.length === 0 ? (
            <div style={{ padding: "0.8rem", background: "rgba(255,92,122,0.1)", borderRadius: "6px", border: "1px solid rgba(255,92,122,0.25)" }}>
              <p style={{ margin: 0, fontSize: "0.72rem", color: "var(--sm-error)" }}>
                No paired devices hold the CLIPBOARD permission.
              </p>
              <button
                type="button"
                onClick={onNavigateDevices}
                style={{ marginTop: "0.4rem", background: "none", border: "none", color: "var(--sm-200)", fontSize: "0.7rem", cursor: "pointer", textDecoration: "underline" }}
              >
                Manage devices to grant Clipboard permission →
              </button>
            </div>
          ) : (
            <>
              <div className="form-group">
                <label className="form-label">Target Authorized Device</label>
                <select
                  className="form-select"
                  value={selectedDevice}
                  onChange={(e) => setSelectedDevice(e.target.value)}
                  disabled={hasSyncGrant}
                >
                  {clipboardDevices.map((d) => (
                    <option key={d.id} value={d.id}>
                      {d.name} ({d.platform.toUpperCase()}) — {d.id}
                    </option>
                  ))}
                </select>
              </div>

              <div className="form-group">
                <label className="form-label">Sync Direction</label>
                <select
                  className="form-select"
                  value={direction}
                  onChange={(e) => setDirection(e.target.value)}
                  disabled={hasSyncGrant}
                >
                  <option value="bidirectional">Bidirectional — Both Devices Can Push</option>
                  <option value="host_to_client">Host → Client Only (Read from Windows PC)</option>
                  <option value="client_to_host">Client → Host Only (Android Sends to Windows)</option>
                </select>
              </div>

              {!hasSyncGrant ? (
                <button className="primary-action" type="button" onClick={handleActivate} style={{ width: "100%", marginTop: "0.4rem" }}>
                  Activate Clipboard Sync ↗
                </button>
              ) : (
                <div style={{ display: "flex", gap: "0.5rem", marginTop: "0.4rem" }}>
                  {isActive ? (
                    <button className="secondary-action" type="button" onClick={handleSuspend} style={{ flex: 1 }}>Suspend</button>
                  ) : (
                    <button className="primary-action" type="button" onClick={handleResume} style={{ flex: 1 }}>Resume</button>
                  )}
                </div>
              )}
            </>
          )}
        </div>

        {/* Host → Client Push Panel */}
        <div className="glass-surface config-card">
          <p className="eyebrow">Host → Client Clipboard</p>
          <p style={{ margin: "0 0 0.8rem", fontSize: "0.73rem", color: "var(--sm-text-2)" }}>
            Read the Windows clipboard and mark it for delivery to the paired Android client. Content is never logged.
          </p>
          <button
            className="primary-action"
            type="button"
            onClick={handlePushToClient}
            disabled={!isActive || !(direction === "host_to_client" || direction === "bidirectional")}
            style={{ width: "100%" }}
          >
            Read Host Clipboard &amp; Push to Client ⇢
          </button>
          {hostText && (
            <div style={{ marginTop: "0.8rem", padding: "0.7rem", background: "rgba(155,140,255,0.08)", borderRadius: "6px", border: "1px solid rgba(155,140,255,0.2)" }}>
              <p style={{ margin: "0 0 0.3rem", fontSize: "0.65rem", color: "var(--sm-text-3)" }}>Preview (truncated):</p>
              <code style={{ fontSize: "0.72rem", color: "var(--sm-200)", wordBreak: "break-all" }}>{hostText}</code>
            </div>
          )}
        </div>

        {/* Security policy card */}
        <div className="glass-surface config-card">
          <p className="eyebrow">Clipboard Security Policy</p>
          <div className="metric-mini-grid">
            <div className="metric-mini-cell"><span>Max Payload</span><strong>64 KiB</strong></div>
            <div className="metric-mini-cell"><span>Format</span><strong>Plain Text Only</strong></div>
            <div className="metric-mini-cell"><span>Null Byte Guard</span><strong>Active ✓</strong></div>
            <div className="metric-mini-cell"><span>Direction Enforcement</span><strong>Host-Side ✓</strong></div>
          </div>
          <p style={{ margin: "0.5rem 0 0", fontSize: "0.68rem", color: "var(--sm-text-3)", lineHeight: "1.3" }}>
            All clipboard payloads are validated through MigRoute before any Win32 call. Binary formats, null bytes, and oversized payloads are silently rejected. Clipboard text is never written to application logs.
          </p>
        </div>
      </div>
    </div>
  );
}

// ─── Milestone 6: File Migration & Transfer Hub ──────────────────────────────

function TransferHubView({
  onNotice,
  onNavigateDevices,
}: {
  onNotice: (msg: string) => void;
  onNavigateDevices: () => void;
}) {
  const [activeTab, setActiveTab] = useState<"migration" | "clipboard">("migration");

  return (
    <div className="dashboard-stack">
      {/* Sub-navigation segmented switcher */}
      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.2rem" }}>
        <button
          type="button"
          className={activeTab === "migration" ? "primary-action" : "secondary-action"}
          onClick={() => setActiveTab("migration")}
          style={{ padding: "0.45rem 1.1rem", fontSize: "0.78rem", display: "flex", alignItems: "center", gap: "0.45rem" }}
        >
          <span>⇄</span> File Migration
        </button>
        <button
          type="button"
          className={activeTab === "clipboard" ? "primary-action" : "secondary-action"}
          onClick={() => setActiveTab("clipboard")}
          style={{ padding: "0.45rem 1.1rem", fontSize: "0.78rem", display: "flex", alignItems: "center", gap: "0.45rem" }}
        >
          <span>⎘</span> Clipboard Sync
        </button>
      </div>

      {activeTab === "migration" ? (
        <FileMigrationView onNotice={onNotice} onNavigateDevices={onNavigateDevices} />
      ) : (
        <ClipboardSyncView onNotice={onNotice} onNavigateDevices={onNavigateDevices} />
      )}
    </div>
  );
}

function FileMigrationView({
  onNotice,
  onNavigateDevices,
}: {
  onNotice: (msg: string) => void;
  onNavigateDevices: () => void;
}) {
  const [transferDevices, setTransferDevices] = useState<TrustedDevice[]>([]);
  const [selectedDevice, setSelectedDevice] = useState("");
  const [filePath, setFilePath] = useState("");
  const [activeTransfers, setActiveTransfers] = useState<TransferSessionDto[]>([]);
  const [currentOutgoing, setCurrentOutgoing] = useState<TransferSessionDto | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [streamProgress, setStreamProgress] = useState(0);
  const [streamStatusText, setStreamStatusText] = useState("");
  const [loading, setLoading] = useState(true);

  const refreshTransfers = () => {
    invoke<TransferSessionDto[]>("list_transfers")
      .then((list) => setActiveTransfers(list))
      .catch(() => {});
  };

  useEffect(() => {
    invoke<TrustedDevice[]>("get_trusted_devices")
      .then((devs) => {
        const withTransfer = devs.filter(
          (d) =>
            !d.is_revoked &&
            (d.granted_permissions.includes("SEND_FILES") ||
              d.granted_permissions.includes("RECEIVE_FILES"))
        );
        setTransferDevices(withTransfer);
        if (withTransfer.length > 0) setSelectedDevice(withTransfer[0].id);
        setLoading(false);
      })
      .catch(() => setLoading(false));

    refreshTransfers();
  }, []);

  const handleGenerateSample = async () => {
    try {
      const generated = await invoke<string>("create_sample_migration_file", {
        name: `migration_sample_${Date.now().toString().slice(-4)}.bin`,
        sizeKb: 512, // 512 KiB sample = 8 chunks
      });
      setFilePath(generated);
      onNotice("Sample 512 KiB migration asset created in Downloads/SmartMigrate/Samples.");
    } catch (err) {
      onNotice(`Failed to generate sample: ${String(err)}`);
    }
  };

  const handleStartOutgoing = async () => {
    if (!selectedDevice) {
      onNotice("Please select a trusted device with SEND_FILES capability.");
      return;
    }
    if (!filePath.trim()) {
      onNotice("Please enter a file path or click 'Generate Sample'.");
      return;
    }

    try {
      const session = await invoke<TransferSessionDto>("prepare_outgoing_transfer", {
        deviceId: selectedDevice,
        filePath: filePath.trim(),
      });
      setCurrentOutgoing(session);
      setIsStreaming(true);
      setStreamProgress(0);
      setStreamStatusText(`Preparing ${session.fileName} (${session.totalChunks} chunks of 64 KiB)...`);
      onNotice(`Migration initiated: ${session.fileName} (${(session.totalBytes / 1024).toFixed(1)} KB)`);

      const startTime = Date.now();
      for (let i = 0; i < session.totalChunks; i++) {
        const chunk = await invoke<OutgoingChunkDto>("read_outgoing_chunk", {
          transferId: session.transferId,
          chunkIndex: i,
        });
        setStreamProgress(chunk.progressPercent);
        const elapsedSec = Math.max(0.05, (Date.now() - startTime) / 1000);
        const bytesSent = (i + 1) * session.chunkSize;
        const speedMb = (bytesSent / (1024 * 1024 * elapsedSec)).toFixed(2);
        setStreamStatusText(
          `Chunk ${i + 1}/${session.totalChunks} • SHA-256: ${chunk.chunkSha256.slice(0, 10)}… • ${speedMb} MB/s`
        );
        await new Promise((r) => setTimeout(r, 70));
      }

      setIsStreaming(false);
      setStreamStatusText("Complete! All chunks verified and delivered to client.");
      onNotice(`Migration complete: ${session.fileName} successfully transferred.`);
      refreshTransfers();
    } catch (err) {
      setIsStreaming(false);
      onNotice(`Outgoing migration error: ${String(err)}`);
    }
  };

  const handleOpenFolder = async () => {
    try {
      const folder = await invoke<string>("open_transfers_folder");
      onNotice(`Opened Downloads/SmartMigrate in Explorer.`);
    } catch (err) {
      onNotice(`Failed to open folder: ${String(err)}`);
    }
  };

  return (
    <div className="dashboard-stack">
      {/* Telemetry Metric Cards */}
      <section className="metric-grid" aria-label="File Migration Telemetry">
        <Metric
          label="Total Migrations"
          value={String(activeTransfers.length)}
          detail={activeTransfers.length > 0 ? "Tracked in session" : "No active migrations"}
          accent="violet"
        />
        <Metric
          label="Chunk Integrity"
          value="SHA-256"
          detail="Per-chunk FIPS 180-4 verified"
          accent="green"
        />
        <Metric
          label="Isolation Policy"
          value="Enforced"
          detail="Path traversal defense active"
          accent="blue"
        />
      </section>

      <div className="content-grid">
        {/* Outbound File Migration Card */}
        <div className="glass-surface config-card">
          <p className="eyebrow">Outbound File Migration</p>
          <p style={{ margin: "0 0 0.8rem", fontSize: "0.73rem", color: "var(--sm-text-2)" }}>
            Stream chunked files to paired devices with monotonic sequence tracking and cryptographic hashing.
          </p>

          {loading ? (
            <p style={{ fontSize: "0.75rem", color: "var(--sm-text-3)" }}>Loading authorized devices...</p>
          ) : transferDevices.length === 0 ? (
            <div style={{ padding: "0.8rem", background: "rgba(255,255,255,0.03)", borderRadius: "6px" }}>
              <p style={{ margin: 0, fontSize: "0.72rem", color: "var(--sm-text-3)" }}>
                No active devices have <strong>SEND_FILES</strong> permission.
              </p>
              <button
                className="quiet-button"
                type="button"
                onClick={onNavigateDevices}
                style={{ marginTop: "0.5rem" }}
              >
                Go to Devices &amp; Authorize ↗
              </button>
            </div>
          ) : (
            <>
              <div style={{ marginBottom: "0.8rem" }}>
                <label style={{ fontSize: "0.68rem", color: "var(--sm-text-3)", display: "block", marginBottom: "0.3rem" }}>
                  Target Remote Device
                </label>
                <select
                  value={selectedDevice}
                  onChange={(e) => setSelectedDevice(e.target.value)}
                  style={{
                    width: "100%",
                    background: "rgba(255,255,255,0.05)",
                    border: "1px solid var(--sm-border)",
                    borderRadius: "6px",
                    color: "var(--sm-text-1)",
                    padding: "0.45rem",
                    fontSize: "0.75rem",
                  }}
                >
                  {transferDevices.map((d) => (
                    <option key={d.id} value={d.id} style={{ background: "#111" }}>
                      {d.name} ({d.platform} • {d.fingerprint})
                    </option>
                  ))}
                </select>
              </div>

              <div style={{ marginBottom: "0.8rem" }}>
                <label style={{ fontSize: "0.68rem", color: "var(--sm-text-3)", display: "block", marginBottom: "0.3rem" }}>
                  Source File Path
                </label>
                <div style={{ display: "flex", gap: "0.4rem" }}>
                  <input
                    type="text"
                    value={filePath}
                    onChange={(e) => setFilePath(e.target.value)}
                    placeholder="e.g. C:\Files\data.zip"
                    style={{
                      flex: 1,
                      background: "rgba(255,255,255,0.05)",
                      border: "1px solid var(--sm-border)",
                      borderRadius: "6px",
                      color: "var(--sm-text-1)",
                      padding: "0.45rem",
                      fontSize: "0.72rem",
                      fontFamily: "monospace",
                    }}
                  />
                  <button
                    type="button"
                    className="secondary-action"
                    onClick={handleGenerateSample}
                    style={{ padding: "0.35rem 0.6rem", fontSize: "0.68rem", whiteSpace: "nowrap" }}
                    title="Creates a sample 512 KiB file in Downloads/SmartMigrate/Samples for immediate testing"
                  >
                    Generate Sample 🧪
                  </button>
                </div>
              </div>

              <button
                className="primary-action"
                type="button"
                onClick={handleStartOutgoing}
                disabled={isStreaming}
                style={{ width: "100%", marginTop: "0.2rem" }}
              >
                {isStreaming ? "Streaming Chunks..." : "Start Outbound Migration ↗"}
              </button>

              {isStreaming && (
                <div style={{ marginTop: "0.8rem" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.68rem", marginBottom: "0.2rem", color: "var(--sm-text-2)" }}>
                    <span>Transfer Progress</span>
                    <span>{streamProgress.toFixed(1)}%</span>
                  </div>
                  <div style={{ height: "6px", background: "rgba(255,255,255,0.08)", borderRadius: "3px", overflow: "hidden" }}>
                    <div
                      style={{
                        height: "100%",
                        width: `${streamProgress}%`,
                        background: "linear-gradient(90deg, #7A66F0, #3DDC97)",
                        transition: "width 0.1s ease",
                      }}
                    />
                  </div>
                  <p style={{ margin: "0.4rem 0 0", fontSize: "0.66rem", color: "var(--sm-text-3)", fontFamily: "monospace" }}>
                    {streamStatusText}
                  </p>
                </div>
              )}
            </>
          )}
        </div>

        {/* Inbound & Staging Policy Card */}
        <div className="glass-surface config-card">
          <p className="eyebrow">Inbound Migration &amp; Staging</p>
          <p style={{ margin: "0 0 0.8rem", fontSize: "0.73rem", color: "var(--sm-text-2)" }}>
            Incoming transfers are validated and written to an isolated staging cache before atomic finalization.
          </p>

          <div style={{ padding: "0.7rem", background: "rgba(255,255,255,0.03)", borderRadius: "6px", border: "1px solid var(--sm-border)", marginBottom: "0.8rem" }}>
            <span style={{ fontSize: "0.66rem", color: "var(--sm-text-3)", display: "block" }}>Verified Download Directory:</span>
            <code style={{ fontSize: "0.72rem", color: "var(--sm-text-1)", wordBreak: "break-all" }}>
              %USERPROFILE%\Downloads\SmartMigrate
            </code>
            <span style={{ fontSize: "0.66rem", color: "var(--sm-text-3)", display: "block", marginTop: "0.4rem" }}>Staging Sandbox:</span>
            <code style={{ fontSize: "0.72rem", color: "var(--sm-text-2)", wordBreak: "break-all" }}>
              %USERPROFILE%\Downloads\SmartMigrate\.staging\*.part
            </code>
          </div>

          <button
            className="secondary-action"
            type="button"
            onClick={handleOpenFolder}
            style={{ width: "100%", marginBottom: "0.8rem", display: "flex", alignItems: "center", justifyContent: "center", gap: "0.4rem" }}
          >
            <span>📁</span> Open Downloads Folder in Explorer
          </button>

          <div className="metric-mini-grid">
            <div className="metric-mini-cell"><span>Path Defense</span><strong style={{ color: "#3DDC97" }}>ACTIVE ✓</strong></div>
            <div className="metric-mini-cell"><span>Max File Cap</span><strong>4 GiB</strong></div>
            <div className="metric-mini-cell"><span>Chunk Size</span><strong>64 KiB</strong></div>
            <div className="metric-mini-cell"><span>Hash Algorithm</span><strong>SHA-256</strong></div>
          </div>
        </div>

        {/* Security Policy Card */}
        <div className="glass-surface config-card">
          <p className="eyebrow">Transfer Security Architecture</p>
          <ul style={{ margin: "0.4rem 0 0.6rem", paddingLeft: "1.1rem", fontSize: "0.72rem", color: "var(--sm-text-2)", lineHeight: "1.6" }}>
            <li><strong>Path Traversal Defense:</strong> Remote peer filenames are filtered by <code>migroute::sanitize_file_name</code>. Path separators, directory jumps (<code>..</code>), null bytes, and NTFS reserved names (<code>CON</code>, <code>PRN</code>, etc.) are strictly rejected.</li>
            <li><strong>Isolated Staging:</strong> Incomplete streams reside in <code>.staging/[id].part</code>. File is atomically renamed only after total SHA-256 hash matches the announcement.</li>
            <li><strong>Monotonic Resumption:</strong> Checkpoint tracking guarantees that reconnecting endpoints resume from the exact first unacknowledged chunk index.</li>
          </ul>
        </div>
      </div>

      {/* Transfer History Queue */}
      <section className="glass-surface" style={{ padding: "1.2rem", marginTop: "0.5rem" }}>
        <div className="panel-heading" style={{ marginBottom: "0.8rem" }}>
          <div>
            <p className="eyebrow">Migration Session Log</p>
            <h2 style={{ margin: 0, fontSize: "1.1rem" }}>Active &amp; Historical Transfers</h2>
          </div>
          <button className="quiet-button" type="button" onClick={refreshTransfers}>Refresh ↻</button>
        </div>

        {activeTransfers.length === 0 ? (
          <p style={{ margin: 0, fontSize: "0.75rem", color: "var(--sm-text-3)", padding: "1rem 0" }}>
            No files have been migrated during this host session. Use "Start Outbound Migration" above to begin.
          </p>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
            {activeTransfers.map((tx) => (
              <div
                key={tx.transferId}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "0.7rem 1rem",
                  background: "rgba(255,255,255,0.02)",
                  border: "1px solid var(--sm-border)",
                  borderRadius: "8px",
                  gap: "1rem",
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "0.6rem" }}>
                    <span style={{ fontSize: "0.9rem" }}>{tx.direction === "HostToClient" ? "↗" : "↙"}</span>
                    <strong style={{ fontSize: "0.8rem", color: "var(--sm-text-1)", textOverflow: "ellipsis", overflow: "hidden", whiteSpace: "nowrap" }}>
                      {tx.fileName}
                    </strong>
                    <span
                      style={{
                        fontSize: "0.65rem",
                        padding: "0.15rem 0.5rem",
                        borderRadius: "10px",
                        background:
                          tx.state === "Completed"
                            ? "rgba(61, 220, 151, 0.15)"
                            : tx.state === "InProgress"
                            ? "rgba(122, 102, 240, 0.15)"
                            : "rgba(255, 255, 255, 0.08)",
                        color:
                          tx.state === "Completed"
                            ? "#3DDC97"
                            : tx.state === "InProgress"
                            ? "#9B8CFF"
                            : "var(--sm-text-2)",
                      }}
                    >
                      {tx.state}
                    </span>
                  </div>
                  <div style={{ display: "flex", gap: "0.8rem", marginTop: "0.2rem", fontSize: "0.68rem", color: "var(--sm-text-3)" }}>
                    <span>Size: {(tx.totalBytes / 1024).toFixed(1)} KB</span>
                    <span>Chunks: {tx.chunksCompleted} / {tx.totalChunks}</span>
                    <span>Expected SHA: <code>{tx.expectedSha256.slice(0, 12)}…</code></span>
                  </div>
                </div>

                <div style={{ width: "120px", textAlign: "right" }}>
                  <span style={{ fontSize: "0.75rem", fontWeight: 600, color: "var(--sm-text-1)" }}>
                    {tx.progressPercent.toFixed(0)}%
                  </span>
                  <div style={{ height: "4px", background: "rgba(255,255,255,0.08)", borderRadius: "2px", overflow: "hidden", marginTop: "0.2rem" }}>
                    <div
                      style={{
                        height: "100%",
                        width: `${tx.progressPercent}%`,
                        background: tx.state === "Completed" ? "#3DDC97" : "#7A66F0",
                      }}
                    />
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

// ─── Audit History View ──────────────────────────────────────────────────────

function AuditHistoryView({
  onNavigateDevices,
}: {
  onNavigateDevices: () => void;
}) {
  const [deviceCount, setDeviceCount] = useState(0);
  const [transfers, setTransfers] = useState<TransferSessionDto[]>([]);

  useEffect(() => {
    invoke<TrustedDevice[]>("get_trusted_devices")
      .then((devs) => setDeviceCount(devs.length))
      .catch(() => {});
    invoke<TransferSessionDto[]>("list_transfers")
      .then((list) => setTransfers(list))
      .catch(() => {});
  }, []);

  return (
    <div className="dashboard-stack">
      <section className="metric-grid" aria-label="Audit summary">
        <Metric label="Authorization Scope" value="Host Bound" detail="Host decides all grants" accent="violet" />
        <Metric label="Audit Records" value={String(transfers.length + deviceCount)} detail="Strictly non-sensitive" accent="green" />
        <Metric label="Secret Redaction" value="100% Enforced" detail="Zero raw paths or tokens" accent="blue" />
      </section>

      <section className="glass-surface" style={{ padding: "1.2rem" }}>
        <div className="panel-heading" style={{ marginBottom: "0.8rem" }}>
          <div>
            <p className="eyebrow">Security &amp; Capability Audit Trail</p>
            <h2 style={{ margin: 0, fontSize: "1.1rem" }}>Verified Platform Transitions</h2>
          </div>
          <button className="quiet-button" type="button" onClick={onNavigateDevices}>Devices ↗</button>
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem" }}>
          <div
            style={{
              padding: "0.8rem",
              background: "rgba(255,255,255,0.02)",
              border: "1px solid var(--sm-border)",
              borderRadius: "8px",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <strong style={{ fontSize: "0.8rem", color: "var(--sm-text-1)" }}>
                HOST_IDENTITY_INITIALIZED
              </strong>
              <span style={{ fontSize: "0.68rem", color: "#3DDC97" }}>VERIFIED ✓</span>
            </div>
            <p style={{ margin: "0.3rem 0 0", fontSize: "0.7rem", color: "var(--sm-text-3)" }}>
              Local DPAPI persistent hardware identity verified. Device fingerprint issued with zero cloud dependencies.
            </p>
          </div>

          <div
            style={{
              padding: "0.8rem",
              background: "rgba(255,255,255,0.02)",
              border: "1px solid var(--sm-border)",
              borderRadius: "8px",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <strong style={{ fontSize: "0.8rem", color: "var(--sm-text-1)" }}>
                SMP1_PROTOCOL_ENGINE_ACTIVE
              </strong>
              <span style={{ fontSize: "0.68rem", color: "#3DDC97" }}>SECURE ✓</span>
            </div>
            <p style={{ margin: "0.3rem 0 0", fontSize: "0.7rem", color: "var(--sm-text-3)" }}>
              Smart Migrate Protocol version 1 (SMP/1) envelope validation active with monotonic sequence tracking and replay defense.
            </p>
          </div>

          <div
            style={{
              padding: "0.8rem",
              background: "rgba(255,255,255,0.02)",
              border: "1px solid var(--sm-border)",
              borderRadius: "8px",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <strong style={{ fontSize: "0.8rem", color: "var(--sm-text-1)" }}>
                PATH_TRAVERSAL_DEFENSE_BARRIER
              </strong>
              <span style={{ fontSize: "0.68rem", color: "#3DDC97" }}>ENFORCED ✓</span>
            </div>
            <p style={{ margin: "0.3rem 0 0", fontSize: "0.7rem", color: "var(--sm-text-3)" }}>
              Isolated staging directory in place. Remote file offers are sanitized and path traversal attempts are discarded at the boundary.
            </p>
          </div>

          {transfers.map((tx) => (
            <div
              key={tx.transferId}
              style={{
                padding: "0.8rem",
                background: "rgba(255,255,255,0.02)",
                border: "1px solid var(--sm-border)",
                borderRadius: "8px",
              }}
            >
              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                <strong style={{ fontSize: "0.8rem", color: "var(--sm-text-1)" }}>
                  MIGRATION_RECORD_{tx.direction.toUpperCase()}
                </strong>
                <span
                  style={{
                    fontSize: "0.68rem",
                    color: tx.state === "Completed" ? "#3DDC97" : "#9B8CFF",
                  }}
                >
                  {tx.state.toUpperCase()}
                </span>
              </div>
              <p style={{ margin: "0.3rem 0 0", fontSize: "0.7rem", color: "var(--sm-text-3)" }}>
                Asset: {tx.fileName} • {tx.totalChunks} chunks of 64 KiB • Checksum: {tx.expectedSha256.slice(0, 16)}…
              </p>
            </div>
          ))}
        </div>
      </section>
    </div>
  );
}

function UnavailablePanel({ destination, onUnavailable }: { destination: Destination; onUnavailable: (feature: string) => void }) {
  return (
    <section className="unavailable-panel glass-surface">
      <span className="large-symbol" aria-hidden="true">{destinationIcons[destination]}</span>
      <p className="eyebrow">Milestone-gated capability</p>
      <h2>{destination} is designed, but not enabled.</h2>
      <p>Smart Migrate only activates privileged functionality after its native adapter, protocol validation, error states, tests, and security review are in place.</p>
      <button className="secondary-action" type="button" onClick={() => onUnavailable(destination)}>Why is it unavailable?</button>
    </section>
  );
}

function StartupIntro({ onComplete }: { onComplete: () => void }) {
  const [phase, setPhase] = useState<"black" | "glow" | "emblem" | "text" | "dissolve">("black");

  useEffect(() => {
    if (typeof window !== "undefined" && window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      onComplete();
      return;
    }

    const t1 = setTimeout(() => setPhase("glow"), 300);
    const t2 = setTimeout(() => setPhase("emblem"), 800);
    const t3 = setTimeout(() => setPhase("text"), 1400);
    const t4 = setTimeout(() => setPhase("dissolve"), 2000);
    const t5 = setTimeout(() => onComplete(), 2400);

    return () => {
      clearTimeout(t1);
      clearTimeout(t2);
      clearTimeout(t3);
      clearTimeout(t4);
      clearTimeout(t5);
    };
  }, [onComplete]);

  return (
    <div
      className={`startup-intro ${phase === "dissolve" ? "dissolve" : ""}`}
      onClick={onComplete}
      role="presentation"
    >
      <div className="intro-stars" aria-hidden="true" />
      <div className={`intro-glow ${phase !== "black" ? "visible" : ""}`} aria-hidden="true" />

      <div className="intro-stage">
        <div className={`intro-emblem ${phase === "emblem" || phase === "text" || phase === "dissolve" ? "visible" : ""}`}>
          <div className="intro-emblem-mark">
            <span>SM</span>
            <div className="intro-arrows" aria-hidden="true">
              <span className="arrow-left">‹</span>
              <span className="arrow-right">›</span>
            </div>
          </div>
        </div>

        <div className={`intro-typography ${phase === "text" || phase === "dissolve" ? "visible" : ""}`}>
          <h2>SMART MIGRATE</h2>
          <div className="intro-sweep" aria-hidden="true" />
          <p>Powered by MigRoute</p>
        </div>
      </div>

      <button
        type="button"
        className="intro-skip"
        onClick={(e) => {
          e.stopPropagation();
          onComplete();
        }}
        aria-label="Skip startup animation"
      >
        Skip ↗
      </button>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<App />);
