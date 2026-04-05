/**
 * auth.js — session management for dawg.city
 *
 * The session token is stored in localStorage under 'session_token'.
 * On callback the server adds ?token=... to the redirect URL; this
 * script reads it and persists it before cleaning the URL.
 *
 * Exports (via window.Auth):
 *   Auth.getToken()        → string | null
 *   Auth.isLoggedIn()      → boolean
 *   Auth.logout()          → void
 *   Auth.getUser()         → Promise<object | null>
 *   Auth.requireLogin()    → void  (redirects if not logged in)
 *   Auth.renderAuthArea()  → void  (fills #auth-area in topbar)
 */

const API = 'https://dawg.city';   // same-origin; adjust if api subdomain

const Auth = (() => {
  const TOKEN_KEY = 'session_token';
  const USER_KEY  = 'session_user';

  // ── Harvest token from URL after OAuth callback ──────────────────────
  function harvestTokenFromUrl() {
    const params = new URLSearchParams(location.search);
    const token  = params.get('token');
    if (token) {
      localStorage.setItem(TOKEN_KEY, token);
      params.delete('token');
      const clean = params.toString()
        ? `${location.pathname}?${params}`
        : location.pathname;
      history.replaceState(null, '', clean);
    }
  }

  function getToken() {
    return localStorage.getItem(TOKEN_KEY);
  }

  function isLoggedIn() {
    return !!getToken();
  }

  function logout() {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(USER_KEY);
    window.location.href = '/';
  }

  async function getUser() {
    const token = getToken();
    if (!token) return null;

    // Return cached user to avoid repeated round-trips
    const cached = localStorage.getItem(USER_KEY);
    if (cached) {
      try { return JSON.parse(cached); } catch (_) {}
    }

    try {
      const res = await fetch(`${API}/api/me`, {
        headers: { Authorization: `Bearer ${token}` }
      });
      if (!res.ok) {
        if (res.status === 401) {
          // Token expired / invalid — clear it
          localStorage.removeItem(TOKEN_KEY);
          localStorage.removeItem(USER_KEY);
        }
        return null;
      }
      const user = await res.json();
      localStorage.setItem(USER_KEY, JSON.stringify(user));
      return user;
    } catch (_) {
      return null;
    }
  }

  function requireLogin() {
    if (!isLoggedIn()) {
      window.location.href = `${API}/api/auth/discord`;
    }
  }

  /**
   * Fills the element with id="auth-area" in the topbar.
   * Shows a "Login" button or user avatar + coins when logged in.
   */
  async function renderAuthArea() {
    const area = document.getElementById('auth-area');
    if (!area) return;

    if (!isLoggedIn()) {
      area.innerHTML = `
        <a href="${API}/api/auth/discord" class="btn btn-discord">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M20.317 4.37a19.79 19.79 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057c.002.022.015.043.032.054a19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03z"/>
          </svg>
          Login with Discord
        </a>`;
      return;
    }

    const user = await getUser();
    if (!user) {
      area.innerHTML = `<a href="${API}/api/auth/discord" class="btn btn-discord">Login</a>`;
      return;
    }

    const avatar = user.avatar_url
      ? `<img src="${user.avatar_url}" alt="${user.username}">`
      : `<span style="width:28px;height:28px;border-radius:50%;background:#333;display:inline-block"></span>`;

    area.innerHTML = `
      <div class="coin-badge">
        <span class="coin-icon">🪙</span>
        <span id="topbar-coins">${user.coins ?? 0}</span>
      </div>
      <a href="/profile.html" class="avatar-pill" title="${user.username}">
        ${avatar}
        <span class="uname">${user.username}</span>
      </a>
      <button class="btn btn-outline" onclick="Auth.logout()">Out</button>`;
  }

  // Run on every page load
  harvestTokenFromUrl();

  return { getToken, isLoggedIn, logout, getUser, requireLogin, renderAuthArea };
})();

window.Auth = Auth;

// Auto-render the topbar auth area when DOM is ready
document.addEventListener('DOMContentLoaded', () => Auth.renderAuthArea());
