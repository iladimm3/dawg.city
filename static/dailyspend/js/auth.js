/**
 * auth.js — session management for dailyspend.city
 *
 * The session token is stored in localStorage under 'session_token'.
 * It is shared with dawg.city — both sites use the same token to call
 * the dawg.city API.
 *
 * After OAuth the server appends ?token=... to the redirect URL; this
 * script reads it and persists it before cleaning the URL.
 *
 * Exports (via window.Auth):
 *   Auth.getToken()        → string | null
 *   Auth.isLoggedIn()      → boolean
 *   Auth.logout()          → void
 *   Auth.getUser()         → Promise<object | null>
 *   Auth.requireLogin()    → void  (redirects to dawg.city if not logged in)
 *   Auth.renderAuthArea()  → void  (fills #auth-area in topbar)
 */

const API = 'https://dawg.city';

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

    const cached = localStorage.getItem(USER_KEY);
    if (cached) {
      try { return JSON.parse(cached); } catch (_) {}
    }

    try {
      const res = await fetch(`${API}/api/me`, {
        headers: { Authorization: `Bearer ${token}` }
      });
      if (res.status === 401) {
        localStorage.removeItem(TOKEN_KEY);
        localStorage.removeItem(USER_KEY);
        return null;
      }
      if (!res.ok) return null;
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

  async function renderAuthArea() {
    const area = document.getElementById('auth-area');
    if (!area) return;

    if (!isLoggedIn()) {
      area.innerHTML = `
        <a class="btn btn-discord" href="${API}/api/auth/discord">
          Login with Discord
        </a>`;
      return;
    }

    const user = await getUser();
    if (!user) {
      area.innerHTML = `
        <a class="btn btn-discord" href="${API}/api/auth/discord">
          Login with Discord
        </a>`;
      return;
    }

    const avatarHtml = user.avatar_url
      ? `<img src="${escHtml(user.avatar_url)}" alt="avatar">`
      : `<span>🐾</span>`;

    area.innerHTML = `
      <span class="coin-badge">
        <span class="coin-icon">🪙</span>
        <span id="topbar-coins">${user.coins ?? 0}</span>
      </span>
      <a class="avatar-pill" href="/profile.html">
        ${avatarHtml}
        <span class="uname">${escHtml(user.username)}</span>
      </a>
      <button class="btn btn-outline btn-sm" onclick="Auth.logout()">Logout</button>`;
  }

  function escHtml(str) {
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  // Run on load
  harvestTokenFromUrl();

  return { getToken, isLoggedIn, logout, getUser, requireLogin, renderAuthArea };
})();

document.addEventListener('DOMContentLoaded', () => Auth.renderAuthArea());
