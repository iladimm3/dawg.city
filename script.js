    // ── Cookie banner ──
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

    // ── Scan logic ──
    const input        = document.getElementById('url-input');
    const btn          = document.getElementById('analyze-btn');
    const btnText      = document.getElementById('btn-text');
    const btnSpinner   = document.getElementById('btn-spinner');
    const errorBanner  = document.getElementById('error-banner');
    const resultCard   = document.getElementById('result-card');
    const resultThumb  = document.getElementById('result-thumbnail');
    const ringProgress = document.getElementById('ring-progress');
    const ringPct      = document.getElementById('ring-pct');
    const ringVerdict  = document.getElementById('ring-verdict');
    const verdictBanner = document.getElementById('verdict-banner');
    const primaryPct   = document.getElementById('primary-pct');
    const primaryLabel = document.getElementById('primary-label');
    const primaryBar   = document.getElementById('primary-bar');
    const secondaryPct = document.getElementById('secondary-pct');
    const secondaryLabel = document.getElementById('secondary-label');
    const secondaryBar = document.getElementById('secondary-bar');
    const resultDetails = document.getElementById('result-details');

    // Ring circumference for r=65: 2π×65 ≈ 408
    const CIRC = 2 * Math.PI * 65;

    function fillExample(url, el) {
        input.value = url;
        document.querySelectorAll('.strip-pill').forEach(p => p.style.background = '');
        if (el) el.style.background = 'var(--blue-light)';
        analyze();
    }

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

        errorBanner.classList.add('hidden');
        resultCard.classList.add('hidden');
        resultCard.classList.remove('visible');
        setLoading(true);

        try {
            // reCAPTCHA v3
            const recaptchaToken = await new Promise((resolve, reject) => {
                grecaptcha.ready(() => {
                    grecaptcha.execute('6LdrpZIsAAAAAMxHRMOd9pKS0DOxV9Pp7yXPF8UV', { action: 'analyze' })
                        .then(resolve).catch(reject);
                });
            });

            const response = await fetch('/api/analyze', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ url, recaptcha_token: recaptchaToken })
            });

            if (!response.ok) {
                const txt = await response.text();
                let msg = 'Something went wrong';
                try { msg = JSON.parse(txt).error || msg; } catch { msg = txt || msg; }
                throw new Error(msg);
            }

            const data = await response.json();
            const pct = Math.round(data.confidence * 100);
            const inv = 100 - pct;
            const isFake = data.verdict === 'ai_generated';

            // Thumbnail
            resultThumb.src = data.thumbnail || '';

            // Ring
            ringProgress.style.stroke = isFake ? '#e53e3e' : '#22a06b';
            ringProgress.style.strokeDashoffset = CIRC;
            ringPct.textContent = `${pct}%`;
            ringVerdict.textContent = isFake ? 'AI-Generated' : 'Likely Real';
            ringVerdict.className = isFake ? 'fake' : 'real';

            // Verdict banner
            verdictBanner.className = `verdict-banner ${isFake ? 'fake' : 'real'}`;
            verdictBanner.innerHTML = isFake
                ? 'This input is <strong>likely to contain AI-generated or deepfake content</strong>'
                : 'This input is <strong>likely to be authentic, real content</strong>';

            // Score rows
            primaryPct.textContent = `${pct}%`;
            primaryPct.className = `score-pct-big ${isFake ? 'fake' : 'real'}`;
            primaryLabel.textContent = isFake ? 'AI-Generated Content' : 'Real / Authentic Content';
            primaryBar.className = `score-bar-fill ${isFake ? 'fake' : 'real'}`;
            primaryBar.style.width = '0%';

            secondaryPct.textContent = `${inv}%`;
            secondaryLabel.textContent = isFake ? 'Real Content' : 'AI-Generated';
            secondaryBar.style.width = '0%';

            resultDetails.textContent = data.details;

            // Show result + monetization
            resultCard.classList.remove('hidden');
            setTimeout(() => {
                resultCard.classList.add('visible');
                const offset = CIRC - (CIRC * data.confidence);
                ringProgress.style.strokeDashoffset = offset;
                primaryBar.style.width = `${pct}%`;
                secondaryBar.style.width = `${inv}%`;
                document.getElementById('mono-section').classList.remove('hidden');
                resultCard.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }, 60);

            // Increment scan count if logged in
            incrementScanCount();

        } catch (err) {
            showError(err.message);
        } finally {
            setLoading(false);
        }
    }

    btn.addEventListener('click', analyze);
    input.addEventListener('keypress', e => { if (e.key === 'Enter') analyze(); });


    function scanAnother() {
        input.value = '';
        errorBanner.classList.add('hidden');
        resultCard.classList.add('hidden');
        resultCard.classList.remove('visible');
        document.getElementById('mono-section').classList.add('hidden');
        document.querySelectorAll('.strip-pill').forEach(p => p.style.background = '');
        input.scrollIntoView({ behavior: 'smooth', block: 'center' });
        setTimeout(() => input.focus(), 400);
    }

    function openUpgradeModal() {
        document.getElementById('upgrade-modal').classList.add('open');
        document.body.style.overflow = 'hidden';
    }

    function closeUpgradeModal() {
        document.getElementById('upgrade-modal').classList.remove('open');
        document.body.style.overflow = '';
    }

    function handleModalClick(e) {
        if (e.target === document.getElementById('upgrade-modal')) closeUpgradeModal();
    }

    // ── Supabase Auth ──
    const SUPABASE_URL = 'https://uywuhvqsgzxltkvksvjv.supabase.co';
    const SUPABASE_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InV5d3VodnFzZ3p4bHRrdmtzdmp2Iiwicm9sZSI6ImFub24iLCJpYXQiOjE3NzQyMDE0NDMsImV4cCI6MjA4OTc3NzQ0M30.VA_6lwzbU1IHuPpMgIpy7lcA0XiN0Op5PB4Tl3K3JLk';

    _sb = null; // supabase client
    try {
        _sb = window.supabase.createClient(SUPABASE_URL, SUPABASE_KEY);
    } catch(e) {
        console.error('Supabase init failed:', e);
    }

    let currentUser = null;
    let scanCount = 0;

    // Init auth state on load
    async function initAuth() {
        if (!_sb) return;

        // Register listener FIRST before getSession so we catch the PKCE callback
        _sb.auth.onAuthStateChange(async (event, session) => {
            console.log('Auth event:', event, session?.user?.email || '');
            if (event === 'SIGNED_IN' && session?.user) {
                await onSignedIn(session.user);
                document.getElementById('signin-overlay').classList.remove('open');
                document.body.style.overflow = '';
                if (window.location.hash || window.location.search.includes('code=')) {
                    window.history.replaceState({}, document.title, window.location.pathname);
                }
            } else if (event === 'SIGNED_OUT') {
                onSignedOut();
            }
        });

        // Then check for existing session
        const { data: { session }, error } = await _sb.auth.getSession();
        if (error) console.error('Session error:', error);
        if (session?.user) await onSignedIn(session.user);
    }

    function submitEmail() {
        const emailInput = document.getElementById('email-input');
        if (!emailInput) return;
        const email = emailInput.value.trim();
        if (!email || !email.includes('@')) {
            emailInput.style.borderColor = 'var(--red)';
            return;
        }
        document.getElementById('newsletter-success').style.display = 'block';
        emailInput.disabled = true;
        document.querySelector('.email-btn') && (document.querySelector('.email-btn').disabled = true);
    }

    async function onSignedIn(user) {
        currentUser = user;

        // Update nav avatar with initial
        const initial = (user.user_metadata?.full_name || user.email || '?')[0].toUpperCase();
        document.getElementById('nav-avatar').textContent = initial;

        // Load scan count from Supabase
        await loadScanCount(user.id);

        // Show logged-in nav
        document.getElementById('nav-loggedout').classList.add('hidden');
        document.getElementById('nav-loggedin').classList.remove('hidden');
    }

    function onSignedOut() {
        currentUser = null;
        scanCount = 0;
        document.getElementById('nav-loggedout').classList.remove('hidden');
        document.getElementById('nav-loggedin').classList.add('hidden');
        document.getElementById('nav-scan-num').textContent = '0';
    }

    async function loadScanCount(userId) {
        const { data, error } = await _sb
            .from('profiles')
            .select('scan_count')
            .eq('id', userId)
            .single();

        if (data) {
            scanCount = data.scan_count || 0;
            document.getElementById('nav-scan-num').textContent = scanCount;
        } else if (error?.code === 'PGRST116') {
            // Profile doesn't exist yet — create it
            await _sb.from('profiles').insert({ id: userId, scan_count: 0 });
        }
    }

    async function incrementScanCount() {
        if (!currentUser) return;
        scanCount++;
        document.getElementById('nav-scan-num').textContent = scanCount;
        await _sb
            .from('profiles')
            .upsert({ id: currentUser.id, scan_count: scanCount });
    }

    async function signInWithGoogle() {
        if (!_sb) {
            document.getElementById('signin-error').textContent = 'Auth not available. Please try again.';
            document.getElementById('signin-error').classList.remove('hidden');
            return;
        }
        try {
            const { error } = await _sb.auth.signInWithOAuth({
                provider: 'google',
                options: {
                    redirectTo: 'https://dawg.city',
                    flowType: 'pkce'
                }
            });
            if (error) throw error;
        } catch (err) {
            console.error('Google sign-in error:', err);
            document.getElementById('signin-error').textContent = err.message;
            document.getElementById('signin-error').classList.remove('hidden');
        }
    }

    async function signOut() {
        if (!_sb) return;
        await _sb.auth.signOut();
        onSignedOut();
    }

    // Make auth functions globally accessible from onclick attributes
    window.shareOnX = function() {
        const pct = document.getElementById('confidence-pct')?.textContent || '';
        const verdict = document.getElementById('ring-verdict')?.textContent || '';
        const url = document.getElementById('url-input')?.value || '';
        const text = `I just scanned this video on dawg.city 🐕\n\nVerdict: ${verdict} (${pct} confidence)\n\nCheck it yourself 👇\nhttps://dawg.city`;
        window.open(`https://x.com/intent/post?text=${encodeURIComponent(text)}`, '_blank');
    }

    window.copyResult = function() {
        const pct = document.getElementById('confidence-pct')?.textContent || '';
        const verdict = document.getElementById('ring-verdict')?.textContent || '';
        const url = document.getElementById('url-input')?.value || '';
        const text = `dawg.city scan result:\nVerdict: ${verdict} (${pct} confidence)\nVideo: ${url}\nScan yours at https://dawg.city`;
        navigator.clipboard.writeText(text).then(() => {
            const btn = document.querySelector('.share-btn-copy');
            if (btn) { btn.textContent = '✅ Copied!'; setTimeout(() => { btn.innerHTML = '📋 Copy result'; }, 2000); }
        });
    }

    window.toggleFaq = function(btn) {
        const item = btn.parentElement;
        item.classList.toggle('open');
    }

    window.openSignIn = function(e) {
        e.preventDefault();
        document.getElementById('signin-overlay').classList.add('open');
        document.body.style.overflow = 'hidden';
    }

    window.closeAuth = function() {
        document.getElementById('signin-overlay').classList.remove('open');
        document.body.style.overflow = '';
    }

    window.handleAuthClick = function(e, id) {
        if (e.target === document.getElementById(id)) window.closeAuth();
    }

    window.signInWithGoogle = signInWithGoogle;
    window.signOut = signOut;
    window.openUpgradeModal = openUpgradeModal;
    window.closeUpgradeModal = closeUpgradeModal;
    window.acceptCookies = acceptCookies;
    window.dismissCookies = dismissCookies;
    window.submitEmail = submitEmail;
    window.scanAnother = scanAnother;
    window.handleModalClick = handleModalClick;

    // Init on page load
    initAuth();

    document.addEventListener('keydown', e => {
        if (e.key === 'Escape') { closeUpgradeModal(); closeAuth(); }
    });

    // Hide mono-section on new scan
    const origAnalyze = analyze;
    input.addEventListener('input', () => {
        document.getElementById('mono-section').classList.add('hidden');
    });
