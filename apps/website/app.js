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
    navigation.classList.remove("open");
  });
});

document.querySelector("#year").textContent = String(new Date().getFullYear());

const updateNavigationGlass = () => {
  topbar?.classList.toggle("is-scrolled", window.scrollY > 8);
};

updateNavigationGlass();
window.addEventListener("scroll", updateNavigationGlass, { passive: true });
