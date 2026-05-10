(() => {
  let deferredPrompt = null;

  const INSTALL_HIDDEN_KEY = "payflow-install-hidden";
  const root = document.documentElement;
  const standalone =
    window.matchMedia("(display-mode: standalone)").matches ||
    window.navigator.standalone === true;
  const isTouchDevice =
    Number(navigator.maxTouchPoints || 0) > 0 ||
    window.matchMedia("(pointer: coarse)").matches;
  const isiPadOS =
    navigator.platform === "MacIntel" && Number(navigator.maxTouchPoints || 0) > 1;
  const isIos = /iphone|ipad|ipod/i.test(navigator.userAgent) || isiPadOS;

  const installHidden = () => {
    try {
      return window.localStorage.getItem(INSTALL_HIDDEN_KEY) === "true";
    } catch (_error) {
      return false;
    }
  };

  const setInstallHidden = (hidden) => {
    try {
      if (hidden) {
        window.localStorage.setItem(INSTALL_HIDDEN_KEY, "true");
      } else {
        window.localStorage.removeItem(INSTALL_HIDDEN_KEY);
      }
    } catch (_error) {}
  };

  const setInstallAvailable = (available) => {
    if (standalone) {
      root.setAttribute("data-app-installed", "true");
      root.removeAttribute("data-install-available");
      root.removeAttribute("data-install-hidden");
      return;
    }

    root.removeAttribute("data-app-installed");

    if (installHidden()) {
      root.setAttribute("data-install-hidden", "true");
    } else {
      root.removeAttribute("data-install-hidden");
    }

    if (available) {
      root.setAttribute("data-install-available", "true");
    } else {
      root.removeAttribute("data-install-available");
    }
  };

  const helpPanel = () => document.querySelector("[data-install-help]");

  const showHelp = () => {
    const panel = helpPanel();
    if (panel) {
      panel.hidden = false;
    }
  };

  const hideHelp = () => {
    const panel = helpPanel();
    if (panel) {
      panel.hidden = true;
    }
  };

  if ("serviceWorker" in navigator) {
    window.addEventListener("load", () => {
      navigator.serviceWorker.register("./sw.js").catch(() => {});
    });
  }

  window.addEventListener("beforeinstallprompt", (event) => {
    event.preventDefault();
    deferredPrompt = event;
    setInstallAvailable(true);
  });

  window.addEventListener("appinstalled", () => {
    deferredPrompt = null;
    setInstallAvailable(false);
    hideHelp();
  });

  document.addEventListener("click", async (event) => {
    const installButton = event.target.closest("[data-install-app]");
    if (installButton) {
      if (!deferredPrompt) {
        showHelp();
        return;
      }

      installButton.disabled = true;
      deferredPrompt.prompt();
      try {
        await deferredPrompt.userChoice;
      } catch (_error) {
      } finally {
        deferredPrompt = null;
        installButton.disabled = false;
        setInstallAvailable(isIos || isTouchDevice);
      }
      return;
    }

    if (event.target.closest("[data-install-hide]")) {
      setInstallHidden(true);
      setInstallAvailable(isIos || isTouchDevice);
      hideHelp();
      return;
    }

    if (event.target.closest("[data-install-show]")) {
      setInstallHidden(false);
      setInstallAvailable(isIos || isTouchDevice);
      return;
    }

    if (event.target.closest("[data-install-help-close]")) {
      hideHelp();
    }
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      hideHelp();
    }
  });

  setInstallAvailable(isIos || isTouchDevice);
})();
