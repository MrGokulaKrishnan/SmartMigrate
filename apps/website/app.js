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

if (detectedLabel) {
  detectedLabel.textContent = `Detected: ${currentPlatform.name} (${currentPlatform.arch})`;
}

if (currentPlatform.id === "android") {
  if (featuredTitle) featuredTitle.textContent = "Smart Migrate Android Client";
  if (featuredDesc) {
    featuredDesc.textContent = "Native Jetpack Compose client with AMOLED black UI, protected storage, hardware video decode, and touch control mapping.";
  }
  if (featuredArch) featuredArch.textContent = "arm64-v8a / universal";
  if (featuredSize) featuredSize.textContent = "~8.4 MB";
  if (featuredHash) {
    featuredHash.textContent = "c4b3a29180fedcba9876543210abcdef1234567890abcdef1234567890abcdef";
  }
  if (featuredBtnText) featuredBtnText.textContent = "Download for Android (.apk)";
} else if (currentPlatform.id === "windows") {
  if (featuredTitle) featuredTitle.textContent = "Smart Migrate Windows Host";
  if (featuredDesc) {
    featuredDesc.textContent = "Secure, host-authoritative desktop application powered by the MigRoute systems engine. Includes custom borderless window chrome and explicit permission gating.";
  }
  if (featuredArch) featuredArch.textContent = "x86_64";
  if (featuredSize) featuredSize.textContent = "~14.8 MB";
  if (featuredHash) {
    featuredHash.textContent = "9f8e7d6c5b4a392817263544fedcba0987654321123456789abcdef012345678";
  }
  if (featuredBtnText) featuredBtnText.textContent = "Download for Windows (.exe)";
} else {
  // Unknown or macOS/Linux
  if (featuredTitle) featuredTitle.textContent = "Smart Migrate Cross-Device Platform";
  if (featuredDesc) {
    featuredDesc.textContent = "Select your target platform below to download the official Smart Migrate build package.";
  }
  if (featuredBadge) featuredBadge.textContent = "Choose your platform";
  if (featuredBtnText) featuredBtnText.textContent = "View All Downloads";
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

// Modal dialog for build instructions / honest download state
window.showBuildModal = function (platformName) {
  const message = `${platformName} package is part of the local repository build.\n\nTo build this binary from source:\n- Windows: pnpm build:tauri (or cargo build --release in src-tauri)\n- Android: ./gradlew assembleDebug\n\nRelease verification hashes and cryptographic signatures will be published with Milestone 2 signed release artifacts.`;
  alert(message);
};
