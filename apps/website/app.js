const menuButton = document.querySelector(".menu-button");
const navigation = document.querySelector(".navigation");
const topbar = document.querySelector(".topbar");

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
  topbar?.classList.toggle("is-scrolled", window.scrollY > 8);
};

updateNavigationGlass();
window.addEventListener("scroll", updateNavigationGlass, { passive: true });

// ─── Platform Detection & Downloads ──────────────────────────────────────────

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
    return { id: "android", name: "Android", arch: "arm64-v8a", ext: ".apk" };
  }
  if (/win/i.test(platform) || /windows/i.test(ua)) {
    const is64 = /x64|win64|wow64|x86_64/i.test(ua) || /x64|win64/i.test(platform);
    return { id: "windows", name: "Windows", arch: is64 ? "x64" : "x86", ext: ".exe" };
  }
  if (/mac/i.test(platform) || /macintosh/i.test(ua)) {
    return { id: "mac", name: "macOS", arch: "Universal", ext: ".dmg" };
  }
  if (/linux/i.test(platform) || /linux/i.test(ua)) {
    return { id: "linux", name: "Linux", arch: "x86_64", ext: ".AppImage" };
  }
  return { id: "unknown", name: "Unknown OS", arch: "x64", ext: "" };
}

const currentPlatform = detectPlatform();
const detectedLabel = document.querySelector("#detected-os-label");
const featuredTitle = document.querySelector("#featured-title");
const featuredDesc = document.querySelector("#featured-desc");
const featuredArch = document.querySelector("#featured-arch");
const featuredSize = document.querySelector("#featured-size");
const featuredHash = document.querySelector("#featured-hash");
const featuredBtnText = document.querySelector("#featured-btn-text");
const featuredBadge = document.querySelector("#featured-badge");
const featuredActionBtn = document.querySelector("#featured-action-btn");

if (detectedLabel) {
  detectedLabel.textContent = `Detected: ${currentPlatform.name} (${currentPlatform.arch})`;
}

if (currentPlatform.id === "android") {
  if (featuredTitle) featuredTitle.textContent = "Smart Migrate Android Client";
  if (featuredDesc) {
    featuredDesc.textContent = "Native Jetpack Compose client with AMOLED black UI, protected storage, hardware video decode, and touch control mapping.";
  }
  if (featuredArch) featuredArch.textContent = "arm64-v8a / universal";
  if (featuredBtnText) featuredBtnText.textContent = "Download for Android (.apk)";
  if (featuredActionBtn) {
    featuredActionBtn.setAttribute("href", DOWNLOADS.android);
    featuredActionBtn.setAttribute("download", "SmartMigrate.apk");
  }
} else if (currentPlatform.id === "windows") {
  if (featuredTitle) featuredTitle.textContent = "Smart Migrate Windows Host";
  if (featuredDesc) {
    featuredDesc.textContent = "Secure, host-authoritative desktop application powered by the MigRoute systems engine. Includes custom borderless window chrome and explicit permission gating.";
  }
  if (featuredArch) featuredArch.textContent = "x86_64";
  if (featuredBtnText) featuredBtnText.textContent = "Download for Windows (.exe)";
  if (featuredActionBtn) {
    featuredActionBtn.setAttribute("href", DOWNLOADS.windowsExe);
    featuredActionBtn.setAttribute("download", "SmartMigrate-Setup-x64.exe");
  }
} else {
  // macOS / Linux / Unknown
  if (featuredTitle) featuredTitle.textContent = "Smart Migrate Cross-Device Platform";
  if (featuredDesc) {
    featuredDesc.textContent = "Select your target platform below to download the official Smart Migrate release package.";
  }
  if (featuredBadge) featuredBadge.textContent = "Choose your platform";
  if (featuredBtnText) featuredBtnText.textContent = "View All Downloads";
  if (featuredActionBtn) {
    featuredActionBtn.setAttribute("href", "#download-matrix");
    featuredActionBtn.removeAttribute("download");
  }
}

// Platform Filter
const filterButtons = document.querySelectorAll(".filter-btn");
const downloadCards = document.querySelectorAll(".download-card");

filterButtons.forEach((btn) => {
  btn.addEventListener("click", () => {
    filterButtons.forEach((b) => b.classList.remove("active"));
    btn.classList.add("active");
    const target = btn.getAttribute("data-filter");

    downloadCards.forEach((card) => {
      const platform = card.getAttribute("data-platform");
      if (target === "all" || platform === target) {
        card.style.display = "";
      } else {
        card.style.display = "none";
      }
    });
  });
});

// Hydrate hashes, file sizes, and download URLs from release.json
async function hydrateReleaseData() {
  try {
    const res = await fetch("release.json");
    if (!res.ok) return;
    const data = await res.json();
    if (!data.artifacts) return;

    const winExe = data.artifacts["windows-exe"];
    const winMsi = data.artifacts["windows-msi"];
    const androidApk = data.artifacts["android-apk"];

    if (winExe) {
      const elSize = document.querySelector("#win-exe-size");
      const elHash = document.querySelector("#win-exe-hash");
      const btnWinExe = document.querySelector("#btn-win-exe");
      if (elSize) elSize.textContent = winExe.size_human || elSize.textContent;
      if (elHash) elHash.textContent = winExe.sha256 || elHash.textContent;
      const exeUrl = winExe.download_url || DOWNLOADS.windowsExe;
      if (btnWinExe) {
        btnWinExe.setAttribute("href", exeUrl);
      }
      if (currentPlatform.id === "windows") {
        if (featuredSize) featuredSize.textContent = winExe.size_human || featuredSize.textContent;
        if (featuredHash) featuredHash.textContent = winExe.sha256 || featuredHash.textContent;
        if (featuredActionBtn) {
          featuredActionBtn.setAttribute("href", exeUrl);
        }
      }
    }

    if (winMsi) {
      const elSize = document.querySelector("#win-msi-size");
      const elHash = document.querySelector("#win-msi-hash");
      const btnWinMsi = document.querySelector("#btn-win-msi");
      if (elSize) elSize.textContent = winMsi.size_human || elSize.textContent;
      if (elHash) elHash.textContent = winMsi.sha256 || elHash.textContent;
      const msiUrl = winMsi.download_url || DOWNLOADS.windowsMsi;
      if (btnWinMsi) {
        btnWinMsi.setAttribute("href", msiUrl);
      }
    }

    if (androidApk) {
      const elSize = document.querySelector("#android-apk-size");
      const elHash = document.querySelector("#android-apk-hash");
      const btnAndroidApk = document.querySelector("#btn-android-apk");
      if (elSize) elSize.textContent = androidApk.size_human || elSize.textContent;
      if (elHash) elHash.textContent = androidApk.sha256 || elHash.textContent;
      const apkUrl = androidApk.download_url || DOWNLOADS.android;
      if (btnAndroidApk) {
        btnAndroidApk.setAttribute("href", apkUrl);
      }
      if (currentPlatform.id === "android") {
        if (featuredSize) featuredSize.textContent = androidApk.size_human || featuredSize.textContent;
        if (featuredHash) featuredHash.textContent = androidApk.sha256 || featuredHash.textContent;
        if (featuredActionBtn) {
          featuredActionBtn.setAttribute("href", apkUrl);
        }
      }
    }
  } catch (err) {
    console.debug("Offline or local release metadata hydration skipped:", err);
  }
}

hydrateReleaseData();

// ─── Interactive Particle Canvas Field (Ported from KnowToMigrate) ─────────
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

  const dots = Array.from({ length: 65 }, () => ({
    x: Math.random() * canvas.width,
    y: Math.random() * canvas.height,
    r: Math.random() * 1.6 + 0.6,
    vx: (Math.random() - 0.5) * 0.25,
    vy: (Math.random() - 0.5) * 0.25,
    alpha: Math.random() * 0.35 + 0.1,
    color: Math.random() > 0.35 ? "124, 77, 255" : "56, 189, 248",
  }));

  let mouseX = -1000;
  let mouseY = -1000;
  window.addEventListener("mousemove", (e) => {
    mouseX = e.clientX;
    mouseY = e.clientY;
  }, { passive: true });

  const draw = () => {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
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

    for (let i = 0; i < dots.length; i++) {
      for (let j = i + 1; j < dots.length; j++) {
        const dx = dots[i].x - dots[j].x;
        const dy = dots[i].y - dots[j].y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < 120) {
          ctx.beginPath();
          ctx.moveTo(dots[i].x, dots[i].y);
          ctx.lineTo(dots[j].x, dots[j].y);
          ctx.strokeStyle = `rgba(124, 77, 255, ${0.08 * (1 - dist / 120)})`;
          ctx.lineWidth = 0.6;
          ctx.stroke();
        }
      }

      // Mouse interactive connection
      const mdx = dots[i].x - mouseX;
      const mdy = dots[i].y - mouseY;
      const mdist = Math.sqrt(mdx * mdx + mdy * mdy);
      if (mdist < 140) {
        ctx.beginPath();
        ctx.moveTo(dots[i].x, dots[i].y);
        ctx.lineTo(mouseX, mouseY);
        ctx.strokeStyle = `rgba(56, 189, 248, ${0.2 * (1 - mdist / 140)})`;
        ctx.lineWidth = 0.8;
        ctx.stroke();
      }
    }
    animId = requestAnimationFrame(draw);
  };
  draw();
}

initParticleField();
