/**
 * games.js — fetch games from API and render the game grid for dawg.city
 *
 * Usage:
 *   Games.renderGrid('#game-grid', { category, search, featured });
 *   Games.renderSkeletons('#game-grid', 12);
 */

const API = 'https://dawg.city';

const Games = (() => {

  // ── Fetch games from the API ─────────────────────────────────────────
  async function fetchGames({ category, search, featured } = {}) {
    const params = new URLSearchParams();
    if (category) params.set('category', category);
    if (search)   params.set('search', search);
    if (featured) params.set('featured', 'true');

    const url = `${API}/api/games${params.toString() ? '?' + params : ''}`;
    try {
      const res = await fetch(url);
      if (!res.ok) return [];
      return await res.json();
    } catch (_) {
      return [];
    }
  }

  // ── Build a single game card element ────────────────────────────────
  function buildCard(game) {
    const a = document.createElement('a');
    a.className  = 'game-card';
    a.href       = `/game.html?game=${encodeURIComponent(game.slug)}`;
    a.title      = game.title;

    const thumbContent = game.thumbnail
      ? `<img src="${game.thumbnail}" alt="${game.title}" loading="lazy">`
      : `<div class="thumb-placeholder">🎮</div>`;

    const featuredBadge = game.is_featured
      ? `<span class="badge-featured">⭐ Featured</span>`
      : '';

    const plays = game.play_count > 999
      ? `${(game.play_count / 1000).toFixed(1)}k`
      : game.play_count;

    a.innerHTML = `
      <div class="thumb-wrap">${thumbContent}</div>
      ${featuredBadge}
      <div class="card-body">
        <div class="card-title">${escHtml(game.title)}</div>
        <div class="card-meta">
          <span class="cat-tag">${escHtml(game.category)}</span>
          <span>▶ ${plays}</span>
        </div>
      </div>`;

    return a;
  }

  // ── Render skeleton loading cards ────────────────────────────────────
  function renderSkeletons(selector, count = 12) {
    const container = document.querySelector(selector);
    if (!container) return;
    container.innerHTML = Array.from({ length: count }, () => `
      <div class="skeleton-card">
        <div class="skel-thumb"></div>
        <div class="skel-line"></div>
        <div class="skel-line short"></div>
      </div>`).join('');
  }

  // ── Render the grid ──────────────────────────────────────────────────
  async function renderGrid(selector, filters = {}) {
    const container = document.querySelector(selector);
    if (!container) return;

    renderSkeletons(selector, 12);
    const games = await fetchGames(filters);

    container.innerHTML = '';

    if (!games.length) {
      container.innerHTML = `
        <div class="empty-state">
          <div class="empty-icon">🎮</div>
          <p>No games found. Check back soon!</p>
        </div>`;
      return;
    }

    for (const game of games) {
      container.appendChild(buildCard(game));
    }
  }

  // ── Minimal XSS-safe escaper ─────────────────────────────────────────
  function escHtml(str) {
    return str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#039;');
  }

  return { fetchGames, renderGrid, renderSkeletons, buildCard };
})();

window.Games = Games;
