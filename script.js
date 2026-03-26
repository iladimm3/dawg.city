// ── Cookie banner ──────────────────────────────────────────────────
function acceptCookies() {
    localStorage.setItem('cookie_consent', 'accepted');
    document.getElementById('cookie-banner').style.display = 'none';
}
function dismissCookies() {
    localStorage.setItem('cookie_consent', 'declined');
    document.getElementById('cookie-banner').style.display = 'none';
}
if (localStorage.getItem('cookie_consent')) {
    document.getElementById('cookie-banner').style.display = 'none';
}

// ── Constants ──────────────────────────────────────────────────────
const SUPABASE_URL = 'https://uywuhvqsgzxltkvksvjv.supabase.co';
const SUPABASE_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InV5d3VodnFzZ3p4bHRrdmtzdmp2Iiwicm9sZSI6ImFub24iLCJpYXQiOjE3NzQyMDE0NDMsImV4cCI6MjA4OTc3NzQ0M30.VA_6lwzbU1IHuPpMgIpy7lcA0XiN0Op5PB4Tl3K3JLk';
const SCAN_LIMITS  = { free: 5, starter: 50, pro: Infinity };
const CIRC         = 2 * Math.PI * 65;

// ── DOM refs ───────────────────────────────────────────────────────
const input          = document.getElementById('url-input');
const btn            = document.getElementById('analyze-btn');
const btnText        = document.getElementById('btn-text');
const btnSpinner     = document.getElementById('btn-spinner');
const errorBanner    = document.getElementById('error-banner');
const resultCard     = document.getElementById('result-card');
const resultThumb    = document.getElementById('result-thumbnail');
const ringProgress   = document.getElementById('ring-progress');
const ringPct        = document.getElementById('ring-pct');
const ringVerdict    = document.getElementById('ring-verdict');
const verdictBanner  = document.getElementById('verdict-banner');
const primaryPct     = document.getElementById('primary-pct');
const primaryLabel   = document.getElementById('primary-label');
const primaryBar     = document.getElementById('primary-bar');
const secondaryPct   = document.getElementById('secondary-pct');
const secondaryLabel = document.getElementById('secondary-label');
const secondaryBar   = document.getElementById('secondary-bar');
const resultDetails  = document.getElementById('result-details');

// ── Supabase ───────────────────────────────────────────────────────
let _sb = null;
try { _sb = window.supabase.createClient(SUPABASE_URL, SUPABASE_KEY); }
catch(e) { console.error('Supabase init failed:', e); }

let currentUser    = null;
let userPlan       = 'free';
let scanCountMonth = 0;

// ── Plan helpers ───────────────────────────────────────────────────
function isPaid() { return userPlan === 'starter' || userPlan === 'pro'; }

function applyPlanUI() {
    document.querySelectorAll('.ad-wrap, .adsbygoogle').forEach(el => {
        el.style.display = isPaid() ? 'none' : '';
    });
    const upgradeBanner = document.querySelector('.upgrade-wrap');
    if (upgradeBanner) upgradeBanner.style.display = isPaid() ? 'none' : '';

    const scanCountEl = document.getElementById('nav-scan-num');
    if (scanCountEl) {
        const limit = SCAN_LIMITS[userPlan] ?? SCAN_LIMITS.free;
        scanCountEl.textContent = userPlan === 'pro' ? '∞' : `${scanCountMonth}/${limit}`;
    }

    let badge = document.getElementById('nav-plan-badge');
    if (isPaid()) {
        if (!badge) {
            badge = document.createElement('span');
            badge.id = 'nav-plan-badge';
            badge.style.cssText = `font-size:0.68rem;font-weight:700;letter-spacing:0.08em;text-transform:uppercase;padding:3px 10px;border-radius:100px;background:${userPlan === 'pro' ? 'linear-gradient(135deg,#6366f1,#8b5cf6)' : 'var(--blue)'};color:#fff;margin-left:4px;`;
            const navUser = document.getElementById('nav-loggedin');
            if (navUser) navUser.prepend(badge);
        }
        badge.textContent = userPlan === 'pro' ? '⚡ Pro' : '★ Starter';
    } else if (badge) {
        badge.remove();
    }
}

function checkMonthlyReset(resetDateStr) {
    if (!resetDateStr) return false;
    const resetDate = new Date(resetDateStr);
    const now = new Date();
    return resetDate.getMonth() !== now.getMonth() || resetDate.getFullYear() !== now.getFullYear();
}

async function loadProfile(userId) {
    const { data, error } = await _sb
        .from('profiles')
        .select('scan_count, scan_count_month, scan_reset_date, plan')
        .eq('id', userId)
        .single();

    if (data) {
        userPlan = data.plan || 'free';
        if (checkMonthlyReset(data.scan_reset_date)) {
            scanCountMonth = 0;
            await _sb.from('profiles').upsert({ id: userId, scan_count_month: 0, scan_reset_date: new Date().toISOString().split('T')[0] });
        } else {
            scanCountMonth = data.scan_count_month || 0;
        }
        applyPlanUI();
    } else if (error?.code === 'PGRST116') {
        await _sb.from('profiles').insert({ id: userId, scan_count: 0, scan_count_month: 0, plan: 'free', scan_reset_date: new Date().toISOString().split('T')[0] });
        userPlan = 'free'; scanCountMonth = 0; applyPlanUI();
    }
}

function incrementScanCount() {
    if (!currentUser) return;
    scanCountMonth++;
    applyPlanUI();
    // fire-and-forget: server already tracked the quota via increment_scan_quota RPC
    _sb.from('profiles').upsert({ id: currentUser.id, scan_count_month: scanCountMonth, scan_count: scanCountMonth }).then().catch(() => {});
}

// ── Auth ───────────────────────────────────────────────────────────
async function onSignedIn(user) {
    currentUser = user;
    const initial = (user.user_metadata?.full_name || user.email || '?')[0].toUpperCase();
    document.getElementById('nav-avatar').textContent = initial;
    await loadProfile(user.id);
    document.getElementById('nav-loggedout').classList.add('hidden');
    document.getElementById('nav-loggedin').classList.remove('hidden');
}

function onSignedOut() {
    currentUser = null; userPlan = 'free'; scanCountMonth = 0;
    document.getElementById('nav-loggedout').classList.remove('hidden');
    document.getElementById('nav-loggedin').classList.add('hidden');
    document.getElementById('nav-scan-num').textContent = '0';
    applyPlanUI();
}

async function initAuth() {
    if (!_sb) return;
    _sb.auth.onAuthStateChange(async (event, session) => {
        if (event === 'SIGNED_IN' && session?.user) {
            await onSignedIn(session.user);
            document.getElementById('signin-overlay').classList.remove('open');
            document.body.style.overflow = '';
            if (window.location.hash || window.location.search.includes('code='))
                window.history.replaceState({}, document.title, window.location.pathname);
        } else if (event === 'SIGNED_OUT') { onSignedOut(); }
    });
    const { data: { session }, error } = await _sb.auth.getSession();
    if (error) console.error('Session error:', error);
    if (session?.user) await onSignedIn(session.user);
}

// ── Scan ───────────────────────────────────────────────────────────
function setLoading(on) {
    btn.disabled = on;
    btnText.classList.toggle('hidden', on);
    btnSpinner.classList.toggle('hidden', !on);
}

function showError(msg) {
    errorBanner.textContent = msg;
    errorBanner.classList.remove('hidden');
    errorBanner.scrollIntoView({ behavior: 'smooth', block: 'center' });
}

async function analyze() {
    const url = input.value.trim();
    if (!url) { input.focus(); return; }

    // Enforce monthly limit for signed-in users
    if (currentUser && userPlan !== 'pro') {
        const limit = SCAN_LIMITS[userPlan] ?? SCAN_LIMITS.free;
        if (scanCountMonth >= limit) {
            showError(`You've used all ${limit} scans this month. Upgrade for more!`);
            openUpgradeModal();
            return;
        }
    }

    // Guest limit: 3 scans via localStorage
    if (!currentUser) {
        const guestScans = parseInt(localStorage.getItem('guest_scans') || '0');
        if (guestScans >= 3) {
            showError("You've used your 3 free guest scans. Sign in for more!");
            document.getElementById('signin-overlay').classList.add('open');
            document.body.style.overflow = 'hidden';
            return;
        }
    }

    errorBanner.classList.add('hidden');
    resultCard.classList.add('hidden');
    resultCard.classList.remove('visible');
    setLoading(true);

    try {
        let recaptchaToken = null;
        try {
            if (typeof grecaptcha === 'undefined') {
                throw new Error('reCAPTCHA not loaded');
            }
            const RECAPTCHA_TIMEOUT_MS = 8000;
            recaptchaToken = await Promise.race([
                new Promise((resolve, reject) => {
                    try {
                        grecaptcha.ready(() => {
                            grecaptcha.execute('6LdIIZgsAAAAAB599FU3Jyyq3a8dcTSOodDsbjiC', { action: 'analyze' })
                                .then(resolve).catch(reject);
                        });
                    } catch (e) { reject(e); }
                }),
                new Promise((_, reject) =>
                    setTimeout(() => reject(new Error('reCAPTCHA timed out')), RECAPTCHA_TIMEOUT_MS)
                )
            ]);
        } catch (e) {
            showError('reCAPTCHA unavailable — please disable your adblocker or try again.');
            throw e;
        }

        const headers = { 'Content-Type': 'application/json' };
        if (currentUser && _sb) {
            const { data: { session } } = await _sb.auth.getSession();
            if (session?.access_token) {
                headers['Authorization'] = `Bearer ${session.access_token}`;
            }
        }

        const controller = new AbortController();
        const fetchTimeout = setTimeout(() => controller.abort(), 12000);

        let response;
        try {
            response = await fetch('/api/analyze', {
                method: 'POST',
                headers,
                body: JSON.stringify({ url, recaptcha_token: recaptchaToken }),
                signal: controller.signal
            });
        } catch (e) {
            if (e.name === 'AbortError') throw new Error('Request timed out — the server took too long. Please try again.');
            throw e;
        } finally {
            clearTimeout(fetchTimeout);
        }

        if (!response.ok) {
            const txt = await response.text();
            let msg = 'Something went wrong';
            try { msg = JSON.parse(txt).error || msg; } catch { msg = txt || msg; }
            throw new Error(msg);
        }

        const data   = await response.json();
        const pct    = Math.round(data.confidence * 100);
        const inv    = 100 - pct;
        const isFake = data.verdict === 'ai_generated';

        resultThumb.src = data.thumbnail || '';
        ringProgress.style.stroke = isFake ? '#e53e3e' : '#22a06b';
        ringProgress.style.strokeDashoffset = CIRC;
        ringPct.textContent = `${pct}%`;
        ringVerdict.textContent = isFake ? 'AI-Generated' : 'Likely Real';
        ringVerdict.className = isFake ? 'fake' : 'real';
        verdictBanner.className = `verdict-banner ${isFake ? 'fake' : 'real'}`;
        verdictBanner.innerHTML = isFake
            ? 'This input is <strong>likely to contain AI-generated or deepfake content</strong>'
            : 'This input is <strong>likely to be authentic, real content</strong>';
        primaryPct.textContent = `${pct}%`;
        primaryPct.className = `score-pct-big ${isFake ? 'fake' : 'real'}`;
        primaryLabel.textContent = isFake ? 'AI-Generated Content' : 'Real / Authentic Content';
        primaryBar.className = `score-bar-fill ${isFake ? 'fake' : 'real'}`;
        primaryBar.style.width = '0%';
        secondaryPct.textContent = `${inv}%`;
        secondaryLabel.textContent = isFake ? 'Real Content' : 'AI-Generated';
        secondaryBar.style.width = '0%';
        resultDetails.textContent = data.details;

        resultCard.classList.remove('hidden');
        setTimeout(() => {
            resultCard.classList.add('visible');
            ringProgress.style.strokeDashoffset = CIRC - (CIRC * data.confidence);
            primaryBar.style.width = `${pct}%`;
            secondaryBar.style.width = `${inv}%`;
            document.getElementById('mono-section').classList.remove('hidden');
            resultCard.scrollIntoView({ behavior: 'smooth', block: 'start' });
            initInArticleAd();
        }, 60);

        if (currentUser) { incrementScanCount(); }
        else { localStorage.setItem('guest_scans', parseInt(localStorage.getItem('guest_scans') || '0') + 1); }

    } catch (err) { showError(err.message); }
    finally { setLoading(false); }
}

btn.addEventListener('click', analyze);
input.addEventListener('keypress', e => { if (e.key === 'Enter') analyze(); });
input.addEventListener('input', () => { document.getElementById('mono-section').classList.add('hidden'); });

// ── UI helpers ─────────────────────────────────────────────────────
function scanAnother() {
    input.value = '';
    errorBanner.classList.add('hidden');
    resultCard.classList.add('hidden');
    resultCard.classList.remove('visible');
    document.getElementById('mono-section').classList.add('hidden');
    input.scrollIntoView({ behavior: 'smooth', block: 'center' });
    setTimeout(() => input.focus(), 400);
}
function openUpgradeModal() { document.getElementById('upgrade-modal').classList.add('open'); document.body.style.overflow = 'hidden'; }
function closeUpgradeModal() { document.getElementById('upgrade-modal').classList.remove('open'); document.body.style.overflow = ''; }
function handleModalClick(e) { if (e.target === document.getElementById('upgrade-modal')) closeUpgradeModal(); }
function submitEmail() {
    const emailInput = document.getElementById('email-input');
    if (!emailInput) return;
    const email = emailInput.value.trim();
    if (!email || !email.includes('@')) { emailInput.style.borderColor = 'var(--red)'; return; }
    document.getElementById('newsletter-success').style.display = 'block';
    emailInput.disabled = true;
}
function fillExample(url, el) {
    input.value = url;
    document.querySelectorAll('.strip-pill').forEach(p => p.style.background = '');
    if (el) el.style.background = 'var(--blue-light)';
    analyze();
}

// ── Auth actions ───────────────────────────────────────────────────
async function signInWithGoogle() {
    if (!_sb) return;
    try {
        const { error } = await _sb.auth.signInWithOAuth({ provider: 'google', options: { redirectTo: 'https://dawg.city', flowType: 'pkce' } });
        if (error) throw error;
    } catch (err) {
        document.getElementById('signin-error').textContent = err.message;
        document.getElementById('signin-error').classList.remove('hidden');
    }
}
async function signOut() { if (!_sb) return; await _sb.auth.signOut(); onSignedOut(); }

// ── Share ──────────────────────────────────────────────────────────
window.shareOnX = function() {
    const verdict = ringVerdict?.textContent || '';
    const pct     = ringPct?.textContent || '';
    const text    = `I just scanned this video on dawg.city 🐕\n\nVerdict: ${verdict} (${pct} confidence)\n\nCheck it yourself 👇\nhttps://dawg.city`;
    window.open(`https://x.com/intent/post?text=${encodeURIComponent(text)}`, '_blank');
};
window.copyResult = function() {
    const verdict = ringVerdict?.textContent || '';
    const pct     = ringPct?.textContent || '';
    const text    = `dawg.city scan result:\nVerdict: ${verdict} (${pct} confidence)\nVideo: ${input?.value || ''}\nScan yours at https://dawg.city`;
    navigator.clipboard.writeText(text).then(() => {
        const copyBtn = document.querySelector('.share-btn-copy');
        if (copyBtn) { copyBtn.textContent = '✅ Copied!'; setTimeout(() => { copyBtn.innerHTML = '📋 Copy result'; }, 2000); }
    });
};

// ── Global exports ─────────────────────────────────────────────────
window.toggleFaq        = b => b.parentElement.classList.toggle('open');
window.openSignIn       = e => { e.preventDefault(); document.getElementById('signin-overlay').classList.add('open'); document.body.style.overflow = 'hidden'; };
window.closeAuth        = () => { document.getElementById('signin-overlay').classList.remove('open'); document.body.style.overflow = ''; };
window.handleAuthClick  = (e, id) => { if (e.target === document.getElementById(id)) window.closeAuth(); };
window.signInWithGoogle = signInWithGoogle;
window.signOut          = signOut;
window.openUpgradeModal = openUpgradeModal;
window.closeUpgradeModal= closeUpgradeModal;
window.acceptCookies    = acceptCookies;
window.dismissCookies   = dismissCookies;
window.submitEmail      = submitEmail;
window.scanAnother      = scanAnother;
window.handleModalClick = handleModalClick;
window.fillExample      = fillExample;

document.addEventListener('keydown', e => { if (e.key === 'Escape') { closeUpgradeModal(); window.closeAuth(); } });

// ── AdSense init (deferred to avoid width=0 on hidden elements) ────
function pushAd(el) {
    if (!el || el.dataset.adPushed) return;
    try {
        const w = el.closest('.ad-wrap') || el.parentElement;
        if (w && w.offsetWidth === 0) return; // still hidden, skip
        (window.adsbygoogle = window.adsbygoogle || []).push({});
        el.dataset.adPushed = 'true';
    } catch (e) { /* AdSense errors are non-fatal */ }
}

window.addEventListener('load', () => {
    // Push top banner ad only (it's always visible)
    document.querySelectorAll('.adsbygoogle:not([data-ad-lazy])').forEach(pushAd);
});

// in-article ad (inside result-card) is pushed when results are shown
const _origShowResult = window._showResultCard;
function initInArticleAd() {
    document.querySelectorAll('.adsbygoogle[data-ad-lazy]').forEach(pushAd);
}

initAuth();
