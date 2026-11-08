/* global window, document, navigator, IntersectionObserver, fetch */

const REPO = 'jpenren/erglite';
const RELEASE_PAGE = `https://github.com/${REPO}/releases/latest`;

/* -------------------------------------------------------------------------- */
/* Header and mobile navigation                                                */
/* -------------------------------------------------------------------------- */

function initHeader() {
  const header = document.querySelector('[data-header]');
  const toggle = document.querySelector('[data-nav-toggle]');

  if (header) {
    const updateHeader = () => {
      header.dataset.scrolled = window.scrollY > 10 ? 'true' : 'false';
    };
    updateHeader();
    window.addEventListener('scroll', updateHeader, { passive: true });
  }

  if (!header || !toggle) return;

  const setOpen = (open) => {
    header.dataset.open = open ? 'true' : 'false';
    toggle.setAttribute('aria-expanded', String(open));
    toggle.setAttribute('aria-label', open ? 'Close menu' : 'Open menu');
    document.body.classList.toggle('nav-open', open);

    if (open) {
      const firstLink = header.querySelector('.site-nav a');
      if (firstLink) firstLink.focus({ preventScroll: true });
    }
  };

  toggle.addEventListener('click', () => {
    setOpen(header.dataset.open !== 'true');
  });

  header.querySelectorAll('.site-nav a, .header-cta').forEach((link) => {
    link.addEventListener('click', () => {
      if (header.dataset.open === 'true') setOpen(false);
    });
  });

  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && header.dataset.open === 'true') {
      setOpen(false);
      toggle.focus();
    }
  });

  window.addEventListener('resize', () => {
    if (window.innerWidth > 980 && header.dataset.open === 'true') setOpen(false);
  });
}

/* -------------------------------------------------------------------------- */
/* Scroll reveal                                                              */
/* -------------------------------------------------------------------------- */

function initReveal() {
  const elements = document.querySelectorAll('.reveal');
  if (!elements.length) return;

  const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  if (reduceMotion || !('IntersectionObserver' in window)) {
    elements.forEach((element) => element.classList.add('is-visible'));
    return;
  }

  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        entry.target.classList.add('is-visible');
        observer.unobserve(entry.target);
      });
    },
    { rootMargin: '0px 0px -8% 0px', threshold: 0.08 },
  );

  elements.forEach((element) => observer.observe(element));
}

/* -------------------------------------------------------------------------- */
/* Platform tabs                                                              */
/* -------------------------------------------------------------------------- */

function detectPlatform() {
  const ua = navigator.userAgent;
  if (/Windows|Win32|Win64/i.test(ua)) return 'windows';
  if (/Mac OS X|Macintosh/i.test(ua)) return 'macos';
  if (/Linux|X11|CrOS/i.test(ua)) return 'linux';
  return null;
}

function initPlatformTabs() {
  const tabs = Array.from(document.querySelectorAll('[data-platform]'));
  const panels = Array.from(document.querySelectorAll('[data-panel]'));
  if (!tabs.length || !panels.length) return;

  const tabList = tabs[0].parentElement;

  const activate = (platform, focus = false) => {
    tabs.forEach((tab) => {
      const active = tab.dataset.platform === platform;
      tab.setAttribute('aria-selected', String(active));
      tab.tabIndex = active ? 0 : -1;
      if (active && focus) tab.focus();
    });

    panels.forEach((panel) => {
      panel.hidden = panel.dataset.panel !== platform;
    });

    const labels = {
      windows: 'Download for Windows',
      macos: 'Download for macOS',
      linux: 'Download for Linux',
    };
    const heroLabel = document.querySelector('[data-hero-download-label]');
    if (heroLabel && labels[platform]) heroLabel.textContent = labels[platform];

    document.documentElement.dataset.platform = platform;
  };

  tabs.forEach((tab, index) => {
    tab.addEventListener('click', () => activate(tab.dataset.platform));

    tab.addEventListener('keydown', (event) => {
      let nextIndex = null;
      if (event.key === 'ArrowRight') nextIndex = (index + 1) % tabs.length;
      if (event.key === 'ArrowLeft') nextIndex = (index - 1 + tabs.length) % tabs.length;
      if (event.key === 'Home') nextIndex = 0;
      if (event.key === 'End') nextIndex = tabs.length - 1;
      if (nextIndex === null) return;
      event.preventDefault();
      activate(tabs[nextIndex].dataset.platform, true);
    });
  });

  if (tabList) {
    tabList.addEventListener('keydown', (event) => {
      if (!['ArrowRight', 'ArrowLeft', 'Home', 'End'].includes(event.key)) return;
      event.preventDefault();
    });
  }

  const detected = detectPlatform();
  if (detected) activate(detected);
}

/* -------------------------------------------------------------------------- */
/* Latest release assets                                                      */
/* -------------------------------------------------------------------------- */

function initReleaseAssets() {
  const applyRelease = (release) => {
    if (!release || !Array.isArray(release.assets)) return;

    const assets = release.assets;
    const find = (...patterns) => {
      for (const pattern of patterns) {
        const asset = assets.find((candidate) => pattern.test(candidate.name));
        if (asset) return asset;
      }
      return null;
    };

    const byType = {
      'windows-msi': find(/_x64_en-US\.msi$/i, /\.msi$/i),
      'windows-exe': find(/_x64-setup\.exe$/i, /setup\.exe$/i, /\.exe$/i),
      'macos-dmg': find(/universal\.dmg$/i, /aarch64\.dmg$/i, /\.dmg$/i),
      'linux-deb': find(/_amd64\.deb$/i, /\.deb$/i),
      'linux-rpm': find(/x86_64.*\.rpm$/i, /\.rpm$/i),
      'linux-appimage': find(/_amd64\.AppImage$/i, /\.AppImage$/i),
    };

    Object.entries(byType).forEach(([type, asset]) => {
      if (!asset) return;
      document.querySelectorAll(`[data-download="${type}"]`).forEach((link) => {
        link.href = asset.browser_download_url;
      });
    });

    const tag = typeof release.tag_name === 'string' ? release.tag_name.replace(/^v/i, '') : '';
    if (tag) {
      document.querySelectorAll('[data-release-version]').forEach((element) => {
        element.textContent = tag;
      });
    }

    const platform = document.documentElement.dataset.platform || detectPlatform();
    const heroAsset = {
      windows: byType['windows-msi'],
      macos: byType['macos-dmg'],
      linux: byType['linux-appimage'],
    }[platform];

    if (heroAsset) {
      document.querySelectorAll('[data-hero-download]').forEach((link) => {
        link.href = heroAsset.browser_download_url;
      });
    }
  };

  const cached = (() => {
    try {
      const raw = window.sessionStorage.getItem('erglite-release');
      return raw ? JSON.parse(raw) : null;
    } catch {
      return null;
    }
  })();

  if (cached) {
    applyRelease(cached);
    return;
  }

  fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
    headers: { Accept: 'application/vnd.github+json' },
  })
    .then((response) => {
      if (!response.ok) throw new Error(`GitHub API responded with ${response.status}`);
      return response.json();
    })
    .then((release) => {
      applyRelease(release);
      try {
        window.sessionStorage.setItem('erglite-release', JSON.stringify(release));
      } catch {
        /* storage is optional */
      }
    })
    .catch(() => {
      // Keep the release-page fallbacks that are already in the markup.
      document.querySelectorAll('[data-hero-download]').forEach((link) => {
        if (!link.getAttribute('href') || link.getAttribute('href') === '#') {
          link.href = RELEASE_PAGE;
        }
      });
    });
}

/* -------------------------------------------------------------------------- */

function init() {
  initHeader();
  initReveal();
  initPlatformTabs();
  initReleaseAssets();
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init, { once: true });
} else {
  init();
}
