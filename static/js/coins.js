/**
 * coins.js — coin balance display + playtime ping loop for dawg.city
 *
 * Usage on game pages:
 *   Coins.startPingLoop('slope');   // call after DOM ready with the game slug
 *   Coins.stopPingLoop();           // call on beforeunload
 *
 * The balance in the topbar (#topbar-coins) is updated after every
 * successful ping.
 */

const API = 'https://dawg.city';

const Coins = (() => {
  let _pingInterval = null;
  let _currentSlug  = null;

  // ── Toast helper ────────────────────────────────────────────────────
  function toast(msg) {
    let container = document.getElementById('toast-container');
    if (!container) {
      container = document.createElement('div');
      container.id = 'toast-container';
      document.body.appendChild(container);
    }
    const el = document.createElement('div');
    el.className = 'toast';
    el.textContent = msg;
    container.appendChild(el);
    setTimeout(() => el.remove(), 3500);
  }

  // ── Update the topbar coin counter ──────────────────────────────────
  function updateTopbarCoins(delta) {
    const el = document.getElementById('topbar-coins');
    if (!el) return;
    const current = parseInt(el.textContent, 10) || 0;
    el.textContent = current + delta;
  }

  // ── Send a single playtime ping ─────────────────────────────────────
  async function sendPing(slug) {
    const token = window.Auth?.getToken();
    if (!token) return;

    try {
      const res = await fetch(`${API}/api/games/${encodeURIComponent(slug)}/ping`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${token}` }
      });
      if (res.ok) {
        const data = await res.json();
        if (data.awarded) {
          updateTopbarCoins(data.coins ?? 5);
          toast(`+${data.coins ?? 5} 🪙`);
        }
      }
    } catch (_) {
      // Silent fail — never interrupt gameplay
    }
  }

  // ── Start the 60-second ping loop ───────────────────────────────────
  function startPingLoop(slug) {
    if (!slug) return;
    _currentSlug = slug;

    // Don't start if not logged in
    if (!window.Auth?.isLoggedIn()) return;

    // Clear any existing interval
    if (_pingInterval) clearInterval(_pingInterval);

    _pingInterval = setInterval(() => sendPing(_currentSlug), 60_000);
  }

  // ── Stop the ping loop ───────────────────────────────────────────────
  function stopPingLoop() {
    if (_pingInterval) {
      clearInterval(_pingInterval);
      _pingInterval = null;
    }
  }

  // ── Fetch and display coin balance (for profile / coins pages) ───────
  async function fetchBalance() {
    const token = window.Auth?.getToken();
    if (!token) return null;

    try {
      const res = await fetch(`${API}/api/me/coins`, {
        headers: { Authorization: `Bearer ${token}` }
      });
      if (!res.ok) return null;
      return await res.json(); // { balance, history }
    } catch (_) {
      return null;
    }
  }

  // Stop loop when the user closes/navigates away
  window.addEventListener('beforeunload', stopPingLoop);

  return { startPingLoop, stopPingLoop, fetchBalance, toast };
})();

window.Coins = Coins;
