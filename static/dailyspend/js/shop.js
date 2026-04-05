/**
 * shop.js — fetch shop items and handle purchases for dailyspend.city
 *
 * Usage:
 *   Shop.init()            — call on DOMContentLoaded; fetches + renders items
 *   Shop.setFilter(type)   — filter by item type ('', 'badge', 'cosmetic', etc.)
 *   Shop.buyItem(itemId)   — purchase an item by ID (called from card buttons)
 *   Shop.loadItem(itemId)  — load a single item for the detail page
 */

const API = 'https://dawg.city';

const Shop = (() => {
  let _items       = [];
  let _activeType  = '';

  // ── Escape HTML ──────────────────────────────────────────────────────
  function escHtml(str) {
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
  }

  // ── Toast helper ────────────────────────────────────────────────────
  function toast(msg, kind = '') {
    let container = document.getElementById('toast-container');
    if (!container) {
      container = document.createElement('div');
      container.id = 'toast-container';
      document.body.appendChild(container);
    }
    const el = document.createElement('div');
    el.className = `toast${kind ? ' ' + kind : ''}`;
    el.textContent = msg;
    container.appendChild(el);
    setTimeout(() => el.remove(), 3500);
  }

  // ── Update topbar coin counter ───────────────────────────────────────
  function setTopbarCoins(value) {
    const el = document.getElementById('topbar-coins');
    if (el) el.textContent = value;
  }

  // ── Fetch all active items ───────────────────────────────────────────
  async function fetchItems() {
    try {
      const res = await fetch(`${API}/api/shop`);
      if (!res.ok) return [];
      return await res.json();
    } catch (_) {
      return [];
    }
  }

  // ── Fetch a single item by ID ────────────────────────────────────────
  async function fetchItem(id) {
    const items = await fetchItems();
    return items.find(i => i.id === id) ?? null;
  }

  // ── Human readable type label ────────────────────────────────────────
  function typeLabel(type) {
    const map = {
      badge:        '🎖 Badge',
      cosmetic:     '✨ Cosmetic',
      coin_boost:   '🔥 Coin Boost',
      pass_upgrade: '⭐ Pass Upgrade',
    };
    return map[type] ?? type;
  }

  // ── Default emoji for item types ─────────────────────────────────────
  function typeEmoji(type) {
    const map = {
      badge:        '🎖',
      cosmetic:     '✨',
      coin_boost:   '🔥',
      pass_upgrade: '⭐',
    };
    return map[type] ?? '🛒';
  }

  // ── Build a shop item card ───────────────────────────────────────────
  function buildCard(item) {
    const div = document.createElement('div');
    div.className = 'item-card';

    const thumbHtml = item.image_url
      ? `<img src="${escHtml(item.image_url)}" alt="${escHtml(item.name)}" loading="lazy">`
      : typeEmoji(item.item_type);

    const stockOut = item.stock !== null && item.stock !== undefined && item.stock <= 0;

    div.innerHTML = `
      <a href="/item.html?id=${encodeURIComponent(item.id)}" style="display:contents">
        <div class="item-thumb">${thumbHtml}</div>
        <div class="item-body">
          <div class="item-name">${escHtml(item.name)}</div>
          <div class="item-type-tag">${escHtml(typeLabel(item.item_type))}</div>
          <div class="item-price">
            <span class="price-icon">🪙</span>
            ${item.cost_coins.toLocaleString()}
          </div>
          ${stockOut ? '<span class="badge-sold-out">Sold Out</span>' : ''}
        </div>
      </a>
      <div class="item-footer">
        <button
          class="btn-buy"
          ${stockOut ? 'disabled' : ''}
          onclick="event.stopPropagation(); Shop.buyItem('${escHtml(item.id)}')"
        >
          ${stockOut ? 'Sold Out' : `Buy — 🪙 ${item.cost_coins.toLocaleString()}`}
        </button>
      </div>`;

    return div;
  }

  // ── Render skeletons while loading ──────────────────────────────────
  function renderSkeletons(count = 12) {
    const grid = document.getElementById('shop-grid');
    if (!grid) return;
    grid.innerHTML = Array.from({ length: count }, () => `
      <div class="skeleton-item">
        <div class="skel-thumb"></div>
        <div class="skel-line"></div>
        <div class="skel-line short"></div>
      </div>`).join('');
  }

  // ── Render the shop grid ─────────────────────────────────────────────
  function renderGrid(items) {
    const grid = document.getElementById('shop-grid');
    if (!grid) return;
    grid.innerHTML = '';

    if (!items.length) {
      grid.innerHTML = `
        <div class="empty-state" style="grid-column:1/-1">
          <div class="empty-icon">🛒</div>
          <p>No items found.</p>
        </div>`;
      return;
    }

    items.forEach(item => grid.appendChild(buildCard(item)));
  }

  // ── Filter by type ───────────────────────────────────────────────────
  function setFilter(type) {
    _activeType = type;

    // Update pill states
    document.querySelectorAll('.category-bar .pill').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.type === type);
    });

    const filtered = type ? _items.filter(i => i.item_type === type) : _items;
    renderGrid(filtered);
  }

  // ── Purchase an item ─────────────────────────────────────────────────
  async function buyItem(itemId) {
    const token = window.Auth?.getToken();
    if (!token) {
      toast('Log in on dawg.city first to earn and spend coins.', 'error');
      return;
    }

    const item = _items.find(i => i.id === itemId);
    const name = item ? item.name : 'this item';
    const cost = item ? `🪙 ${item.cost_coins.toLocaleString()}` : '';

    if (!confirm(`Spend ${cost} on ${name}?`)) return;

    try {
      const res = await fetch(`${API}/api/shop/buy/${encodeURIComponent(itemId)}`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${token}` }
      });
      const data = await res.json();
      if (data.success) {
        setTopbarCoins(data.new_balance);
        // Refresh cached user so balance stays consistent
        localStorage.removeItem('session_user');
        toast(`Purchased ${name}! 🐾`, 'success');
      } else {
        toast(`Could not purchase: ${data.error ?? 'unknown error'}`, 'error');
      }
    } catch (_) {
      toast('Network error — please try again.', 'error');
    }
  }

  // ── Load the item detail page ────────────────────────────────────────
  async function loadItem(itemId) {
    const item = await fetchItem(itemId);
    if (!item) {
      document.getElementById('item-content').innerHTML = `
        <div class="empty-state">
          <div class="empty-icon">😶</div>
          <p>Item not found.</p>
        </div>`;
      return;
    }

    // Put into shared cache so buyItem() can find it
    _items = [item];

    const thumbHtml = item.image_url
      ? `<img src="${escHtml(item.image_url)}" alt="${escHtml(item.name)}">`
      : typeEmoji(item.item_type);

    const stockOut = item.stock !== null && item.stock !== undefined && item.stock <= 0;
    const stockNote = item.stock === null
      ? 'Unlimited stock'
      : stockOut
        ? 'Out of stock'
        : `${item.stock} remaining`;

    document.title = `${item.name} — dailyspend.city`;

    document.getElementById('item-content').innerHTML = `
      <div class="item-detail-wrap">
        <div class="item-detail-image">${thumbHtml}</div>
        <div class="item-detail-info">
          <div class="detail-type">${escHtml(typeLabel(item.item_type))}</div>
          <h1>${escHtml(item.name)}</h1>
          <p class="detail-desc">${escHtml(item.description ?? 'No description.')}</p>
          <div class="detail-price">
            🪙 ${item.cost_coins.toLocaleString()}
          </div>
          <div class="detail-stock">${escHtml(stockNote)}</div>
          <div class="buy-action">
            <button
              class="btn-buy-lg"
              id="buy-btn"
              ${stockOut ? 'disabled' : ''}
              onclick="Shop.buyItem('${escHtml(item.id)}')"
            >
              ${stockOut ? 'Out of Stock' : `Buy for 🪙 ${item.cost_coins.toLocaleString()}`}
            </button>
            <p class="buy-note">
              Earn coins by playing games on
              <a href="https://dawg.city" style="color:var(--accent)">dawg.city</a>.
            </p>
          </div>
        </div>
      </div>`;
  }

  // ── Init (shop homepage) ─────────────────────────────────────────────
  async function init() {
    renderSkeletons(12);
    _items = await fetchItems();
    renderGrid(_items);
  }

  return { init, setFilter, buyItem, loadItem };
})();
