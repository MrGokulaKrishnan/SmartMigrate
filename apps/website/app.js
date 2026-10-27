/**
 * Smart Migrate — Web Application Script
 * Structural patterns adapted from KnowToMigrate layout architecture.
 * Strictly preserving Smart Migrate's authentic purple/violet palette (#7C4DFF).
 */

// ─── 01 Mobile Navigation & Navbar Scroll Glass ──────────────────────────────
const menuButton = document.querySelector(".menu-button");
const navigation = document.querySelector("#navigation");
const topbar = document.querySelector("#navbar");

menuButton?.addEventListener("click", () => {
  const isOpen = menuButton.getAttribute("aria-expanded") === "true";
  menuButton.setAttribute("aria-expanded", String(!isOpen));
  navigation?.classList.toggle("open", !isOpen);
});

navigation?.querySelectorAll("a").forEach((link) => {
  link.addEventListener("click", () => {
    menuButton?.setAttribute("aria-expanded", "false");
    navigation?.classList.remove("open");
  });
});

const yearEl = document.querySelector("#year");
if (yearEl) {
  yearEl.textContent = String(new Date().getFullYear());
}

const updateNavigationGlass = () => {
  topbar?.classList.toggle("is-scrolled", window.scrollY > 20);
};

updateNavigationGlass();
window.addEventListener("scroll", updateNavigationGlass, { passive: true });

// ─── 02 Platform Detection & Hero CTA ────────────────────────────────────────
const DOWNLOAD_BASE_URL = "/downloads";
const DOWNLOADS = {
  android: `${DOWNLOAD_BASE_URL}/android/SmartMigrate.apk`,
  windowsExe: `${DOWNLOAD_BASE_URL}/windows/SmartMigrate-Setup-x64.exe`,
  windowsMsi: `${DOWNLOAD_BASE_URL}/windows/SmartMigrate-x64.msi`,
};

function detectPlatform() {
  const ua = navigator.userAgent || "";
  const platform = navigator.platform || "";

  if (/android/i.test(ua)) {
    return { id: "android", name: "Android", arch: "arm64-v8a", ext: ".apk", label: "Android (Universal)" };
  }
  if (/win/i.test(platform) || /windows/i.test(ua)) {
    const is64 = /x64|win64|wow64|x86_64/i.test(ua) || /x64|win64/i.test(platform);
    return { id: "windows", name: "Windows", arch: is64 ? "x64" : "x86", ext: ".exe", label: is64 ? "Windows x64" : "Windows x86" };
  }
  if (/mac/i.test(platform) || /macintosh/i.test(ua)) {
    return { id: "mac", name: "macOS", arch: "Universal", ext: ".dmg", label: "macOS" };
  }
  if (/linux/i.test(platform) || /linux/i.test(ua)) {
    return { id: "linux", name: "Linux", arch: "x86_64", ext: ".AppImage", label: "Linux x64" };
  }
  return { id: "unknown", name: "Windows", arch: "x64", ext: ".exe", label: "Windows x64" };
}

const currentPlatform = detectPlatform();
const detectedPlatformLabel = document.querySelector("#hero-detected-platform");
const heroPrimaryCta = document.querySelector("#hero-primary-cta");
const heroCtaText = document.querySelector("#hero-cta-text");

if (detectedPlatformLabel) {
  detectedPlatformLabel.textContent = currentPlatform.label;
}

if (currentPlatform.id === "android") {
  if (heroCtaText) heroCtaText.textContent = "Download for Android (.apk)";
  if (heroPrimaryCta) {
    heroPrimaryCta.setAttribute("href", DOWNLOADS.android);
    heroPrimaryCta.setAttribute("download", "SmartMigrate.apk");
  }
} else {
  // Default to Windows host
  if (heroCtaText) heroCtaText.textContent = "Download for Windows (.exe)";
  if (heroPrimaryCta) {
    heroPrimaryCta.setAttribute("href", DOWNLOADS.windowsExe);
    heroPrimaryCta.setAttribute("download", "SmartMigrate-Setup-x64.exe");
  }
}

// ─── 03 Platform Filter Tabs ─────────────────────────────────────────────────
const filterTabs = document.querySelectorAll(".filter-tab");
const releaseCards = document.querySelectorAll(".download-release-card");

filterTabs.forEach((tab) => {
  tab.addEventListener("click", () => {
    filterTabs.forEach((t) => t.classList.remove("active"));
    tab.classList.add("active");
    const filter = tab.getAttribute("data-filter");

    releaseCards.forEach((card) => {
      const cardPlatform = card.getAttribute("data-platform");
      if (filter === "all" || cardPlatform === filter) {
        card.style.display = "";
      } else {
        card.style.display = "none";
      }
    });
  });
});

// ─── 04 Interactive HiddenHash Component (KnowToMigrate Pattern) ─────────────
const MASKED_STRING = "••••••••••••••••••••••••••••••••";

function setupHiddenHashComponents() {
  const hashBoxes = document.querySelectorAll(".hidden-hash-box");

  hashBoxes.forEach((box) => {
    const toggleBtn = box.querySelector(".hash-toggle-btn");
    const copyBtn = box.querySelector(".hash-copy-btn");
    const codeEl = box.querySelector(".hash-display-code");
    const toggleText = box.querySelector(".toggle-text");
    const copyText = box.querySelector(".copy-text");

    let isUnhidden = false;

    // Toggle Unhide / Hide
    toggleBtn?.addEventListener("click", () => {
      isUnhidden = !isUnhidden;
      const actualHash = box.getAttribute("data-hash") || "";

      if (isUnhidden) {
        if (codeEl) codeEl.textContent = actualHash;
        if (toggleText) toggleText.textContent = "Hide";
        if (copyBtn) copyBtn.style.display = "inline-flex";
        toggleBtn.innerHTML = `
          <svg class="icon-eye-off" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"></path>
            <line x1="1" y1="1" x2="23" y2="23"></line>
          </svg>
          <span class="toggle-text">Hide</span>
        `;
      } else {
        if (codeEl) codeEl.textContent = MASKED_STRING;
        if (toggleText) toggleText.textContent = "Unhide";
        if (copyBtn) copyBtn.style.display = "none";
        toggleBtn.innerHTML = `
          <svg class="icon-eye" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"></path>
            <circle cx="12" cy="12" r="3"></circle>
          </svg>
          <span class="toggle-text">Unhide</span>
        `;
      }
    });

    // Copy to Clipboard
    copyBtn?.addEventListener("click", async () => {
      const actualHash = box.getAttribute("data-hash") || "";
      if (!actualHash) return;

      try {
        await navigator.clipboard.writeText(actualHash);
        copyBtn.innerHTML = `
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="#34d399" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
          <span class="copy-text" style="color: #34d399;">Copied!</span>
        `;
        setTimeout(() => {
          copyBtn.innerHTML = `
            <svg class="icon-copy" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
              <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
            </svg>
            <span class="copy-text">Copy</span>
          `;
        }, 2000);
      } catch (err) {
        console.warn("Clipboard access denied:", err);
      }
    });
  });
}

setupHiddenHashComponents();

// ─── 05 Dynamic Release Metadata & Relative Time Hydration ───────────────────
let cachedReleaseData = null;

function formatRelativeTime(dateInput) {
  if (!dateInput) return "just now";
  const date = new Date(dateInput);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  if (isNaN(diffMs) || diffMs < 45000) return "just now";
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHours = Math.floor(diffMin / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMin <= 1) return "1 minute ago";
  if (diffMin < 60) return `${diffMin} minutes ago`;
  if (diffHours === 1) return "1 hour ago";
  if (diffHours < 24) return `${diffHours} hours ago`;
  if (diffDays === 1) return "yesterday";
  return `${diffDays} days ago`;
}

function updateRelativeTimeDisplay() {
  if (!cachedReleaseData) return;
  const timestamp = cachedReleaseData.updated_at || cachedReleaseData.date;
  if (!timestamp) return;

  const relTime = formatRelativeTime(timestamp);
  const exactDate = formatExactDate(timestamp);

  const bannerTag = document.querySelector("#banner-update-tag");
  if (bannerTag) bannerTag.textContent = `Updated recently: ${relTime}`;

  const heroUpdated = document.querySelector("#hero-updated-time");
  if (heroUpdated) heroUpdated.textContent = `Updated recently (${relTime})`;

  const dlUpdated = document.querySelector("#downloads-last-updated");
  if (dlUpdated) dlUpdated.textContent = `Updated recently: ${relTime}`;

  const dlExact = document.querySelector("#downloads-exact-date");
  if (dlExact) dlExact.textContent = exactDate;

  const winExeDate = document.querySelector("#win-exe-date");
  if (winExeDate) winExeDate.textContent = exactDate;

  const winMsiDate = document.querySelector("#win-msi-date");
  if (winMsiDate) winMsiDate.textContent = exactDate;

  const androidDate = document.querySelector("#android-release-date");
  if (androidDate) androidDate.textContent = `${exactDate} (Updated recently: ${relTime})`;
}

function formatExactDate(dateInput) {
  const date = new Date(dateInput);
  if (isNaN(date.getTime())) return "October 16, 2026";
  const options = { year: "numeric", month: "long", day: "numeric" };
  const dateStr = date.toLocaleDateString("en-US", options);
  const hours = String(date.getUTCHours()).padStart(2, "0");
  const mins = String(date.getUTCMinutes()).padStart(2, "0");
  return `${dateStr} (${hours}:${mins} UTC)`;
}

async function hydrateReleaseData() {
  try {
    let data = null;
    try {
      const mRes = await fetch("releases/manifest.json", { cache: "no-store" });
      if (mRes.ok) {
        const manifest = await mRes.json();
        data = {
          version: manifest.version,
          versionCode: manifest.versionCode,
          updated_at: manifest.releaseDate,
          date: manifest.releaseDate ? manifest.releaseDate.split("T")[0] : null,
          artifacts: {}
        };
        if (manifest.platforms) {
          if (manifest.platforms["windows-x86_64"]) {
            const win = manifest.platforms["windows-x86_64"];
            data.artifacts["windows-exe"] = {
              version: win.version,
              filename: win.fileName || "SmartMigrate-Setup-x64.exe",
              download_url: win.url || "/downloads/windows/SmartMigrate-Setup-x64.exe",
              sha256: win.sha256,
              size_human: win.size ? `${(win.size / 1048576).toFixed(1)} MB` : "4.2 MB"
            };
          }
          if (manifest.platforms["android-arm64"]) {
            const apk = manifest.platforms["android-arm64"];
            data.artifacts["android-apk"] = {
              version: apk.version,
              versionCode: apk.versionCode,
              filename: apk.fileName || "SmartMigrate.apk",
              download_url: apk.url || "/downloads/android/SmartMigrate.apk",
              sha256: apk.sha256,
              size_human: apk.size ? `${(apk.size / 1048576).toFixed(1)} MB` : "2.6 MB"
            };
          }
        }
      }
    } catch (_) {}

    if (!data) {
      const res = await fetch("release.json", { cache: "no-store" });
      if (res.ok) {
        data = await res.json();
      }
    }

    if (!data) return;

    cachedReleaseData = data;
    updateRelativeTimeDisplay();

    if (data.version) {
      const buildStr = data.versionCode ? ` (Build ${data.versionCode})` : "";
      const dlVersion = document.querySelector("#downloads-current-version");
      if (dlVersion) dlVersion.textContent = `v${data.version}${buildStr}`;

      const previewTag = document.querySelector(".preview-brand-tag");
      if (previewTag) previewTag.textContent = `v${data.version}`;

      const winExeVer = document.querySelector("#win-exe-version");
      if (winExeVer) {
        const exeVer = data.artifacts?.["windows-exe"]?.version || data.version;
        winExeVer.textContent = exeVer;
      }

      const winMsiVer = document.querySelector("#win-msi-version");
      if (winMsiVer) {
        const msiVer = data.artifacts?.["windows-msi"]?.version || data.version;
        winMsiVer.textContent = msiVer;
      }

      const androidVer = document.querySelector("#android-apk-version");
      if (androidVer) {
        const apkVer = data.artifacts?.["android-apk"]?.version || data.version;
        androidVer.textContent = apkVer;
      }
    }

    if (!data.artifacts) return;

    const winExe = data.artifacts["windows-exe"];
    const winMsi = data.artifacts["windows-msi"];
    const androidApk = data.artifacts["android-apk"];

    // Windows EXE
    if (winExe) {
      const elSize = document.querySelector("#win-exe-size");
      const btnWinExe = document.querySelector("#btn-win-exe");
      const hashBox = document.querySelector(".download-release-card[data-platform='windows']:first-of-type .hidden-hash-box");

      if (elSize && winExe.size_human) elSize.textContent = winExe.size_human;
      if (btnWinExe && winExe.download_url) btnWinExe.setAttribute("href", winExe.download_url);
      if (hashBox && winExe.sha256) {
        hashBox.setAttribute("data-hash", winExe.sha256);
      }
    }

    // Windows MSI
    if (winMsi) {
      const elSize = document.querySelector("#win-msi-size");
      const btnWinMsi = document.querySelector("#btn-win-msi");
      const hashBox = document.querySelectorAll(".download-release-card[data-platform='windows']")[1]?.querySelector(".hidden-hash-box");

      if (elSize && winMsi.size_human) elSize.textContent = winMsi.size_human;
      if (btnWinMsi && winMsi.download_url) btnWinMsi.setAttribute("href", winMsi.download_url);
      if (hashBox && winMsi.sha256) {
        hashBox.setAttribute("data-hash", winMsi.sha256);
      }
    }

    // Android APK
    if (androidApk) {
      const elSize = document.querySelector("#android-apk-size");
      const btnAndroidApk = document.querySelector("#btn-android-apk");
      const hashBox = document.querySelector(".download-release-card[data-platform='android'] .hidden-hash-box");

      if (elSize && androidApk.size_human) elSize.textContent = androidApk.size_human;
      if (btnAndroidApk && androidApk.download_url) btnAndroidApk.setAttribute("href", androidApk.download_url);
      if (hashBox && androidApk.sha256) {
        hashBox.setAttribute("data-hash", androidApk.sha256);
      }
    }
  } catch (err) {
    console.debug("Offline release metadata hydration skipped:", err);
  }
}

hydrateReleaseData();
setInterval(updateRelativeTimeDisplay, 20000);
setInterval(hydrateReleaseData, 60000);

// ─── 06 Particle Background Canvas (KnowToMigrate Pattern, Purple/Violet) ────
function initParticleField() {
  const canvas = document.getElementById("particle-field");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  let animId;
  const resize = () => {
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
  };
  resize();
  window.addEventListener("resize", resize, { passive: true });

  const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (prefersReducedMotion) return;

  // 60 particle dots in authentic Smart Migrate purple/violet/cyan
  const dots = Array.from({ length: 60 }, () => ({
    x: Math.random() * canvas.width,
    y: Math.random() * canvas.height,
    r: Math.random() * 1.5 + 0.5,
    vx: (Math.random() - 0.5) * 0.22,
    vy: (Math.random() - 0.5) * 0.22,
    alpha: Math.random() * 0.32 + 0.08,
    color: Math.random() > 0.3 ? "124, 77, 255" : "56, 189, 248",
  }));

  let mouseX = -1000;
  let mouseY = -1000;
  window.addEventListener(
    "mousemove",
    (e) => {
      mouseX = e.clientX;
      mouseY = e.clientY;
    },
    { passive: true }
  );

  const draw = () => {
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Draw and update dots
    dots.forEach((d) => {
      d.x += d.vx;
      d.y += d.vy;
      if (d.x < 0) d.x = canvas.width;
      if (d.x > canvas.width) d.x = 0;
      if (d.y < 0) d.y = canvas.height;
      if (d.y > canvas.height) d.y = 0;

      ctx.beginPath();
      ctx.arc(d.x, d.y, d.r, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(${d.color}, ${d.alpha})`;
      ctx.fill();
    });

    // Draw interconnecting network webs
    for (let i = 0; i < dots.length; i++) {
      for (let j = i + 1; j < dots.length; j++) {
        const dx = dots[i].x - dots[j].x;
        const dy = dots[i].y - dots[j].y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < 115) {
          ctx.beginPath();
          ctx.moveTo(dots[i].x, dots[i].y);
          ctx.lineTo(dots[j].x, dots[j].y);
          ctx.strokeStyle = `rgba(124, 77, 255, ${0.07 * (1 - dist / 115)})`;
          ctx.lineWidth = 0.5;
          ctx.stroke();
        }
      }

      // Mouse interactive tethering
      const mdx = dots[i].x - mouseX;
      const mdy = dots[i].y - mouseY;
      const mdist = Math.sqrt(mdx * mdx + mdy * mdy);
      if (mdist < 130) {
        ctx.beginPath();
        ctx.moveTo(dots[i].x, dots[i].y);
        ctx.lineTo(mouseX, mouseY);
        ctx.strokeStyle = `rgba(56, 189, 248, ${0.16 * (1 - mdist / 130)})`;
        ctx.lineWidth = 0.7;
        ctx.stroke();
      }
    }

    animId = requestAnimationFrame(draw);
  };

  draw();
}

initParticleField();
