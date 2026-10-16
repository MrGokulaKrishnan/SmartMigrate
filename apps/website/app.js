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

// ─── 05 Hydrate Metadata & Checksums from release.json ───────────────────────
async function hydrateReleaseData() {
  try {
    const res = await fetch("release.json");
    if (!res.ok) return;
    const data = await res.json();
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
