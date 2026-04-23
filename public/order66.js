(function() {
    var count = 0;
    var lastTime = 0;
    var THRESHOLD = 800;
    var REQUIRED = 5;
    var active = false;
    var ambientInterval = null;
    var quoteInterval = null;
    var starCanvas = null;

    var quotes = [
        '"I find your lack of security disturbing." — Darth Vader',
        '"The dark side of the code is a pathway to many vulnerabilities." — Palpatine',
        '"Do. Or do not. There is no try-catch." — Yoda',
        '"I\'ve got a bad feeling about this deployment." — Han Solo',
        '"These aren\'t the bugs you\'re looking for." — Obi-Wan',
        '"The Force is strong with this scan." — Vader',
        '"Never tell me the odds of a zero-day." — Han Solo',
        '"Your overconfidence is your weakness." — Luke',
        '"Now witness the firepower of this fully armed scanner." — Palpatine',
        '"Judge me by my CVE count, do you?" — Yoda',
        '"I am altering the config. Pray I don\'t alter it further." — Vader',
        '"It\'s a trap! Port 443 is open!" — Admiral Ackbar',
    ];

    function restoreState() {
        if (localStorage.getItem('order66') === 'active') {
            activate();
        }
    }
    if (document.body) {
        restoreState();
    } else {
        document.addEventListener('DOMContentLoaded', restoreState);
    }

    document.addEventListener('keydown', function(e) {
        if (e.key === 'T' && e.shiftKey) {
            e.preventDefault();
            var now = Date.now();
            if (now - lastTime > THRESHOLD) count = 0;
            count++;
            lastTime = now;
            if (count >= REQUIRED) {
                count = 0;
                toggle();
            }
        } else {
            count = 0;
        }
    });

    function toggle() {
        if (active) deactivate();
        else activate();
    }

    function activate() {
        active = true;
        document.body.classList.add('starwars-mode');
        localStorage.setItem('order66', 'active');
        window.dispatchEvent(new CustomEvent('order66toggle', { detail: { active: true } }));

        // Show activation overlay
        if (!document.getElementById('sw-overlay')) {
            var overlay = document.createElement('div');
            overlay.id = 'sw-overlay';
            overlay.innerHTML = '<div class="sw-activate-text">' +
                '<div class="sw-crawl-line">A long time ago in a galaxy far, far away...</div>' +
                '<div class="sw-title">WATCHTOWER</div>' +
                '<div class="sw-subtitle">The Dark Side Edition</div>' +
                '</div>';
            document.body.appendChild(overlay);
            setTimeout(function() {
                overlay.classList.add('sw-fade-out');
                overlay.style.pointerEvents = 'none';
                setTimeout(function() {
                    try { overlay.remove(); } catch(e) {}
                }, 1500);
            }, 3000);
            // Safety: force-remove overlay after 6s no matter what
            setTimeout(function() {
                var stale = document.getElementById('sw-overlay');
                if (stale) try { stale.remove(); } catch(e) {}
            }, 6000);
        }

        playImperialMarch();
        startStarField();
        startQuoteRotation();
        startAmbientSounds();
        // Delay icon swap to avoid conflicts with Leptos hydration
        setTimeout(function() { swapIcon(true); }, 800);
    }

    function deactivate() {
        active = false;
        document.body.classList.remove('starwars-mode');
        localStorage.removeItem('order66');
        window.dispatchEvent(new CustomEvent('order66toggle', { detail: { active: false } }));
        stopStarField();
        stopQuoteRotation();
        stopAmbientSounds();
        swapIcon(false);
    }

    // ── Lightsaber Icon Swap ──
    var swNavMap = {
        '\u{1F4CA}': '\u{1F311}',   // 📊 → 🌑 Death Star
        '\u{1F50D}': '\u26A1',       // 🔍 → ⚡ Force Lightning
        '\u{1F4CB}': '\u{1F4DC}',   // 📋 → 📜 Jedi Archives
        '\u{1F6E0}\uFE0F': '\u{1F680}', // 🛠️ → 🚀 Star Destroyer
        '\u{1F4C4}': '\u{1F4E1}',   // 📄 → 📡 Hologram
        '\u2699\uFE0F': '\u{1F52E}' // ⚙️ → 🔮 Holocron
    };
    var swNavReverse = {};
    Object.keys(swNavMap).forEach(function(k) { swNavReverse[swNavMap[k]] = k; });

    // Module-level state for icon swapping (persists across calls)
    var _swBusy = false;
    var _swTimer = null;
    var _swObs = null;

    function doIconSwap(toSaber) {
        if (_swBusy) return;
        _swBusy = true;
        // Disconnect observer BEFORE touching DOM
        if (_swObs) _swObs.disconnect();

        try {
            // Title icon
            var el = document.querySelector('.sidebar-icon');
            if (el) {
                var want = toSaber ? '\uD83D\uDDE1\uFE0F' : '\u2694\uFE0F';
                if (el.textContent !== want) el.textContent = want;
            }
            // Nav icons
            var navIcons = document.querySelectorAll('.nav-icon');
            navIcons.forEach(function(icon) {
                var txt = icon.textContent.trim();
                if (toSaber && swNavMap[txt]) {
                    icon.textContent = swNavMap[txt];
                } else if (!toSaber && swNavReverse[txt]) {
                    icon.textContent = swNavReverse[txt];
                }
            });
        } catch(e) {}

        // Reconnect observer AFTER a delay so our mutations are fully settled
        setTimeout(function() {
            _swBusy = false;
            if (active && _swObs) {
                var sidebar = document.querySelector('.sidebar');
                if (sidebar) _swObs.observe(sidebar, { childList: true, subtree: true });
            }
        }, 300);
    }

    function swapIcon(toSaber) {
        clearTimeout(_swTimer);
        if (_swObs) { _swObs.disconnect(); _swObs = null; }

        doIconSwap(toSaber);

        if (toSaber) {
            _swObs = new MutationObserver(function() {
                if (!active || _swBusy) return;
                clearTimeout(_swTimer);
                _swTimer = setTimeout(function() { doIconSwap(true); }, 200);
            });
            // Observe only the sidebar, not the entire body
            setTimeout(function() {
                if (!active || !_swObs) return;
                var sidebar = document.querySelector('.sidebar');
                if (sidebar) {
                    _swObs.observe(sidebar, { childList: true, subtree: true });
                }
            }, 500);
        }
    }

    // ── Imperial March ──
    function playImperialMarch() {
        try {
            var ctx = new (window.AudioContext || window.webkitAudioContext)();
            var notes = [392, 392, 392, 311.13, 466.16, 392, 311.13, 466.16, 392];
            var durations = [0.35, 0.35, 0.35, 0.25, 0.1, 0.35, 0.25, 0.1, 0.7];
            var t = ctx.currentTime;
            notes.forEach(function(freq, i) {
                var osc = ctx.createOscillator();
                var gain = ctx.createGain();
                osc.connect(gain);
                gain.connect(ctx.destination);
                osc.frequency.value = freq;
                osc.type = 'square';
                gain.gain.setValueAtTime(0.06, t);
                gain.gain.exponentialRampToValueAtTime(0.001, t + durations[i] * 0.9);
                osc.start(t);
                osc.stop(t + durations[i]);
                t += durations[i];
            });
        } catch(e) {}
    }

    // ── Ambient Sounds (subtle hum + occasional R2-D2 beeps) ──
    function startAmbientSounds() {
        stopAmbientSounds();
        ambientInterval = setInterval(function() {
            if (!active) return;
            if (Math.random() < 0.3) playR2D2Beep();
        }, 15000);
    }

    function stopAmbientSounds() {
        if (ambientInterval) { clearInterval(ambientInterval); ambientInterval = null; }
    }

    function playR2D2Beep() {
        try {
            var ctx = new (window.AudioContext || window.webkitAudioContext)();
            var beeps = [];
            var count = 3 + Math.floor(Math.random() * 4);
            for (var i = 0; i < count; i++) {
                beeps.push({
                    freq: 1200 + Math.random() * 1800,
                    dur: 0.05 + Math.random() * 0.12,
                    gap: 0.03 + Math.random() * 0.06
                });
            }
            var t = ctx.currentTime;
            beeps.forEach(function(b) {
                var osc = ctx.createOscillator();
                var gain = ctx.createGain();
                osc.connect(gain);
                gain.connect(ctx.destination);
                osc.frequency.value = b.freq;
                osc.type = 'sine';
                gain.gain.setValueAtTime(0.03, t);
                gain.gain.exponentialRampToValueAtTime(0.001, t + b.dur * 0.8);
                osc.start(t);
                osc.stop(t + b.dur);
                t += b.dur + b.gap;
            });
        } catch(e) {}
    }

    // ── Star Field Background ──
    function startStarField() {
        stopStarField();
        starCanvas = document.createElement('canvas');
        starCanvas.id = 'sw-starfield';
        starCanvas.style.cssText = 'position:fixed;inset:0;z-index:-1;pointer-events:none;opacity:0.4;';
        document.body.appendChild(starCanvas);

        var ctx = starCanvas.getContext('2d');
        var stars = [];
        var NUM_STARS = 150;

        function resize() {
            starCanvas.width = window.innerWidth;
            starCanvas.height = window.innerHeight;
        }
        resize();
        window.addEventListener('resize', resize);

        for (var i = 0; i < NUM_STARS; i++) {
            stars.push({
                x: Math.random() * starCanvas.width,
                y: Math.random() * starCanvas.height,
                size: Math.random() * 2 + 0.5,
                speed: Math.random() * 0.5 + 0.1,
                brightness: Math.random()
            });
        }

        function draw() {
            if (!starCanvas || !starCanvas.parentNode) return;
            ctx.clearRect(0, 0, starCanvas.width, starCanvas.height);
            stars.forEach(function(s) {
                s.y += s.speed;
                if (s.y > starCanvas.height) {
                    s.y = 0;
                    s.x = Math.random() * starCanvas.width;
                }
                s.brightness += (Math.random() - 0.5) * 0.1;
                s.brightness = Math.max(0.2, Math.min(1, s.brightness));
                ctx.fillStyle = 'rgba(255, 232, 31, ' + (s.brightness * 0.6) + ')';
                ctx.beginPath();
                ctx.arc(s.x, s.y, s.size, 0, Math.PI * 2);
                ctx.fill();
            });
            requestAnimationFrame(draw);
        }
        draw();
    }

    function stopStarField() {
        if (starCanvas && starCanvas.parentNode) {
            starCanvas.parentNode.removeChild(starCanvas);
        }
        starCanvas = null;
    }

    // ── Rotating Quotes in Sidebar ──
    function startQuoteRotation() {
        stopQuoteRotation();
        setQuote();
        quoteInterval = setInterval(setQuote, 12000);
    }

    function stopQuoteRotation() {
        if (quoteInterval) { clearInterval(quoteInterval); quoteInterval = null; }
        // Restore original quote
        var el = document.querySelector('.sidebar-quote');
        if (el && el.dataset.originalQuote) {
            el.textContent = el.dataset.originalQuote;
        }
    }

    function setQuote() {
        var el = document.querySelector('.sidebar-quote');
        if (!el) return;
        if (!el.dataset.originalQuote) {
            el.dataset.originalQuote = el.textContent;
        }
        el.textContent = quotes[Math.floor(Math.random() * quotes.length)];
        el.style.transition = 'opacity 0.5s';
        el.style.opacity = '0';
        setTimeout(function() { el.style.opacity = '1'; }, 50);
    }

    // ── Lightsaber Sound on Button Clicks ──
    document.addEventListener('click', function(e) {
        if (!active) return;
        var target = e.target;
        if (target.classList && (target.classList.contains('btn-primary') || target.classList.contains('btn-lg'))) {
            playLightsaberSound();
        }
    });

    function playLightsaberSound() {
        try {
            var ctx = new (window.AudioContext || window.webkitAudioContext)();
            // Lightsaber ignition: rising frequency sweep
            var osc = ctx.createOscillator();
            var gain = ctx.createGain();
            osc.connect(gain);
            gain.connect(ctx.destination);
            osc.type = 'sawtooth';
            osc.frequency.setValueAtTime(80, ctx.currentTime);
            osc.frequency.exponentialRampToValueAtTime(200, ctx.currentTime + 0.15);
            osc.frequency.exponentialRampToValueAtTime(120, ctx.currentTime + 0.4);
            gain.gain.setValueAtTime(0.04, ctx.currentTime);
            gain.gain.exponentialRampToValueAtTime(0.001, ctx.currentTime + 0.4);
            osc.start(ctx.currentTime);
            osc.stop(ctx.currentTime + 0.4);
        } catch(e) {}
    }
})();
