/**
 * Auto-version: fetches latest release tag from GitHub and updates all
 * elements with class "oa-version" or data-version-prefix attributes.
 *
 * Usage in HTML:
 *   <span class="oa-version">1.0.91</span>           → replaced with latest tag (no "v")
 *   <span class="oa-version-full">v1.0.91</span>     → replaced with "v1.0.91"
 *   <span data-version-prefix="Documentation v">Documentation v1.0.91</span>
 *
 * Falls back to the hardcoded text if fetch fails (offline, rate-limited).
 */
(function () {
  var REPO = 'AnitChaudhry/openanalyst-cli';
  var CACHE_KEY = 'oa_cli_version';
  var CACHE_TTL = 300000; // 5 minutes

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
  }

  // Check sessionStorage cache first
  try {
    var cached = JSON.parse(sessionStorage.getItem(CACHE_KEY));
    if (cached && Date.now() - cached.ts < CACHE_TTL) {
      apply(cached.v);
      return;
    }
  } catch (e) {}

  fetch('https://api.github.com/repos/' + REPO + '/releases/latest')
    .then(function (r) { return r.json(); })
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
