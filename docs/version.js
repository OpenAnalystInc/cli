/**
 * Auto-version: fetches latest release tag from GitHub and updates all
 * elements with class "oa-version" or data-version-prefix attributes.
 *
 * Usage in HTML:
 *   <span class="oa-version">2.0.3</span>           → replaced with latest tag (no "v")
 *   <span class="oa-version-full">v2.0.3</span>     → replaced with "v2.0.3"
 *   <span data-version-prefix="Documentation v">Documentation v2.0.3</span>
 *
 * Falls back to the hardcoded text if fetch fails (offline, rate-limited).
 */
(function () {
  var REPO = 'OpenAnalystInc/cli';
  var DEFAULT_VERSION = 'v2.0.20';
  var CACHE_KEY = 'oa_cli_version';
  var CACHE_TTL = 300000;
  var ICON_URL = 'https://openanalyst.com/images/new-logo.png';

  function apply(version) {
    var bare = version.replace(/^v/, '');
    // .oa-version → bare number
    document.querySelectorAll('.oa-version').forEach(function (el) {
      el.textContent = bare;
    });
    // .oa-version-full → with "v" prefix
    document.querySelectorAll('.oa-version-full').forEach(function (el) {
      el.textContent = version;
    });
    // [data-version-prefix] → prefix + bare
    document.querySelectorAll('[data-version-prefix]').forEach(function (el) {
      el.textContent = el.getAttribute('data-version-prefix') + bare;
    });
    ensureIcons();
    ensureDocsFooter(version);
  }

  function ensureIcons() {
    if (!document.querySelector('link[rel="icon"]')) {
      var icon = document.createElement('link');
      icon.rel = 'icon';
      icon.type = 'image/png';
      icon.href = ICON_URL;
      document.head.appendChild(icon);
    }

    if (!document.querySelector('link[rel="apple-touch-icon"]')) {
      var apple = document.createElement('link');
      apple.rel = 'apple-touch-icon';
      apple.href = ICON_URL;
      document.head.appendChild(apple);
    }
  }

  function ensureDocsFooter(version) {
    var main = document.querySelector('.main-content');
    if (!main || document.querySelector('[data-site-footer="openanalyst"]')) {
      return;
    }

    var footer = document.createElement('div');
    footer.className = 'docs-site-footer';
    footer.setAttribute('data-site-footer', 'openanalyst');
    footer.innerHTML =
      '<span><strong>OpenAnalyst CLI</strong> <span class="oa-version-full">' + version + '</span></span>' +
      '<span>Public docs, install scripts, and release binaries are served from <a href="https://github.com/OpenAnalystInc/cli" target="_blank" rel="noreferrer">OpenAnalystInc/cli</a> and <a href="https://github.com/OpenAnalystInc/cli/releases/latest" target="_blank" rel="noreferrer">latest releases</a></span>';
    main.appendChild(footer);
  }

  apply(DEFAULT_VERSION);

  try {
    var cached = JSON.parse(sessionStorage.getItem(CACHE_KEY));
    if (cached && Date.now() - cached.ts < CACHE_TTL) {
      apply(cached.v);
      return;
    }
  } catch (e) {}

  fetch('https://api.github.com/repos/' + REPO + '/releases/latest', {
    headers: { Accept: 'application/vnd.github+json' }
  })
    .then(function (r) { return r.ok ? r.json() : null; })
    .then(function (data) {
      if (data && data.tag_name) {
        apply(data.tag_name);
        try {
          sessionStorage.setItem(CACHE_KEY, JSON.stringify({ v: data.tag_name, ts: Date.now() }));
        } catch (e) {}
      }
    })
    .catch(function () {
      // Offline or rate-limited — keep hardcoded fallback text
    });
})();
