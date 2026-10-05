import { useEffect, useMemo, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./styles.css";

type Destination = "Home" | "Devices" | "Transfer" | "Remote" | "History" | "Security" | "Settings";

type HostStatus = {
  engine: string;
  platform: string;
  profile: string;
  privilegedFeaturesEnabled: boolean;
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

const previewStatus: HostStatus = {
  engine: "MigRoute",
  platform: "Windows host shell",
  profile: "Local design preview",
  privilegedFeaturesEnabled: false,
};

function App() {
  const [destination, setDestination] = useState<Destination>("Home");
  const [status, setStatus] = useState<HostStatus>(previewStatus);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [notice, setNotice] = useState("Host shell ready. Privileged features remain disabled.");

  useEffect(() => {
    invoke<HostStatus>("host_status")
      .then(setStatus)
      .catch(() => setStatus(previewStatus));
  }, []);

  const title = useMemo(() => (destination === "Home" ? "Good morning, Gokul." : destination), [destination]);

  const showUnavailable = (feature: string) => {
    setNotice(`${feature} is unavailable until its native adapter and security review are complete.`);
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
        <div className="titlebar-state"><span className="state-dot" /> {status.profile}</div>
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
                onClick={() => { setDestination(item); setNotice(`${item} selected.`); }}
              >
                <span aria-hidden="true">{destinationIcons[item]}</span>{item}
              </button>
            ))}
          </nav>
          <div className="sidebar-footer">
            <span className="mini-avatar">G</span>
            <div><strong>Local profile</strong><span>Not signed in</span></div>
          </div>
        </aside>

        <section className="workspace" aria-labelledby="workspace-title">
          <div className="workspace-heading">
            <div>
              <p className="eyebrow">{status.platform}</p>
              <h1 id="workspace-title">{title}</h1>
            </div>
            <button className="primary-action" type="button" onClick={() => setDialogOpen(true)}>Connect device <span aria-hidden="true">＋</span></button>
          </div>

          {destination === "Home" ? (
            <HomeDashboard onUnavailable={showUnavailable} />
          ) : (
            <UnavailablePanel destination={destination} onUnavailable={showUnavailable} />
          )}
        </section>
      </section>

      <p className="live-notice" aria-live="polite">{notice}</p>

      {dialogOpen && (
        <div className="modal-backdrop" role="presentation" onMouseDown={() => setDialogOpen(false)}>
          <section className="pairing-dialog glass-surface" role="dialog" aria-modal="true" aria-labelledby="pairing-title" onMouseDown={(event) => event.stopPropagation()}>
            <span className="dialog-kicker">Pair a device</span>
            <h2 id="pairing-title">Pairing is protected by design.</h2>
            <p>QR and numeric pairing will appear here after the trusted-pairing milestone. This shell never produces a fake code or grants access.</p>
            <button className="secondary-action" type="button" onClick={() => setDialogOpen(false)}>Got it</button>
          </section>
        </div>
      )}
    </main>
  );
}

function HomeDashboard({ onUnavailable }: { onUnavailable: (feature: string) => void }) {
  return (
    <div className="dashboard-stack">
      <section className="host-overview glass-surface">
        <div className="overview-copy">
          <span className="online-badge"><i /> Host shell online</span>
          <h2>This PC is ready for trusted pairing.</h2>
          <p>Smart Migrate will request approval before a device can receive any capability. No active sessions are running.</p>
          <div className="overview-actions">
            <button className="primary-action" type="button" onClick={() => onUnavailable("Device pairing")}>Start pairing <span aria-hidden="true">→</span></button>
            <button className="secondary-action" type="button" onClick={() => onUnavailable("Diagnostics")}>View diagnostics</button>
          </div>
        </div>
        <div className="host-orb" aria-hidden="true"><span>SM</span><i className="orb-route">↗</i></div>
      </section>

      <section className="metric-grid" aria-label="Current host state">
        <Metric label="Trusted devices" value="0" detail="Pair a device to begin" accent="violet" />
        <Metric label="Current session" value="None" detail="Host approval required" accent="blue" />
        <Metric label="Security state" value="Protected" detail="No privileged adapters active" accent="green" />
      </section>

      <section className="content-grid">
        <article className="glass-surface device-panel">
          <div className="panel-heading"><div><p className="eyebrow">Trusted devices</p><h2>No devices yet</h2></div><button className="quiet-button" type="button" onClick={() => onUnavailable("Trusted device management")}>Manage <span aria-hidden="true">↗</span></button></div>
          <div className="empty-device"><div className="device-glyph" aria-hidden="true">◇</div><p>Your approved Windows and Android devices will appear here.</p></div>
        </article>
        <article className="glass-surface security-panel">
          <p className="eyebrow">Connection policy</p>
          <h2>Approval is always visible.</h2>
          <ul>
            <li><span>✓</span> Host decides every capability</li>
            <li><span>✓</span> Sessions expire and can be ended locally</li>
            <li><span>✓</span> Inputs remain disabled until authorized</li>
          </ul>
        </article>
      </section>
    </div>
  );
}

function Metric({ label, value, detail, accent }: { label: string; value: string; detail: string; accent: string }) {
  return <article className={`metric glass-surface ${accent}`}><p>{label}</p><strong>{value}</strong><span>{detail}</span></article>;
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

