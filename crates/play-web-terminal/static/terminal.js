class WebTerminal {
    constructor() {
        this.terminal = null;
        this.fitAddon = null;
        this.ws = null;
        this.isConnected = false;
        this.isHandlingDisconnect = false; // Prevent multiple disconnect handling
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 10;
        this.reconnectDelay = 1000; // Start with 1 second
        
        // Buffer for handling large data streams
        this.outputBuffer = [];
        this.isProcessingBuffer = false;
        
        // Session management
        this.currentSession = null;
        this.tmuxAvailable = false;
        // Scroll strategy for tmux sessions (default: auto)
        this.tmuxScrollMode = 'auto'; // 'auto' | 'dom' | 'tmux'
        
        // Data API client (lazy)
        this.dataClient = null;
        this._dataApiPromise = null;
        this._dataApiTried = false;
        this.sessionHistory = '';
        this.lastSavedSignature = '';
        this.autoSaveInterval = null;
        this.autoSavePeriodMs = 15000; // 15s
        this.MAX_HISTORY_BYTES = 100000; // cap for update payloads
        // Scrolling state to throttle autosave updates (manual only via UI buttons)
        this.isScrolling = false;
        this.scrollIdleTimer = null;
        this.scrollIdleMs = 1200;

        this.initializeTerminal();
        this.setupEventListeners();
        this.setupSessionUI();
        this.setupScrollControls();
        this.connect(); // Auto-connect on load
    }

    // --- ANSI decoding helpers for preview ---
    escapeHtml(str) {
        return str
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;');
    }

    sanitizeAnsiForPreview(str) {
        let s = str;
        // Remove OSC sequences: ESC ] ... BEL or ST
        s = s.replace(/\x1b\][\s\S]*?(?:\x07|\x1b\\)/g, '');
        // Remove CSI sequences that are not SGR ('m')
        s = s.replace(/\x1b\[[0-?]*[ -\/]*[@-~]/g, (m) => (m.endsWith('m') ? m : ''));
        // Remove charset selection ESC ( B, ESC ) 0 etc.
        s = s.replace(/\x1b[\(\)][0-9A-Za-z]/g, '');
        // Remove other single-character ESC sequences (C1 7-bit)
        s = s.replace(/\x1b[@-Z\\-_]/g, '');
        return s;
    }

    // For injection into xterm: keep all cursor/SGR control; strip only OSC sequences (clipboard/title)
    stripOscSequences(str) {
        if (!str) return '';
        return str.replace(/\x1b\][\s\S]*?(?:\x07|\x1b\\)/g, '');
    }

    ansiToHtml(input) {
        if (!input) return '';
        // Normalize newlines
        let text = input.replace(/\r/g, '');
        text = this.sanitizeAnsiForPreview(text);
        const parts = text.split(/\x1b\[/g); // split by ESC[
        // Active style state
        let fg = null, bg = null, bold = false, underline = false, italic = false, inverse = false, faint = false;
        let fgIdx256 = null; // track 256-color index for suggestion detection
        let spanOpen = false;

        // 16/bright color maps
        const colorMap = {
            30: '#000000', 31: '#f38ba8', 32: '#a6e3a1', 33: '#f9e2af', 34: '#89b4fa', 35: '#f5c2e7', 36: '#94e2d5', 37: '#cdd6f4',
            90: '#585b70', 91: '#f38ba8', 92: '#a6e3a1', 93: '#f9e2af', 94: '#89b4fa', 95: '#f5c2e7', 96: '#94e2d5', 97: '#ffffff'
        };
        const bgMap = {
            40: '#000000', 41: '#f38ba8', 42: '#a6e3a1', 43: '#f9e2af', 44: '#89b4fa', 45: '#f5c2e7', 46: '#94e2d5', 47: '#cdd6f4',
            100: '#585b70', 101: '#f38ba8', 102: '#a6e3a1', 103: '#f9e2af', 104: '#89b4fa', 105: '#f5c2e7', 106: '#94e2d5', 107: '#ffffff'
        };

        const toHex = (n) => {
            const h = Math.max(0, Math.min(255, n)).toString(16).padStart(2, '0');
            return h;
        };
        const ansi256ToHex = (idx) => {
            idx = Math.max(0, Math.min(255, idx|0));
            if (idx < 16) {
                // Map first 16 to a reasonable palette similar to xterm defaults
                const base = [
                    '#000000','#800000','#008000','#808000','#000080','#800080','#008080','#c0c0c0',
                    '#808080','#ff0000','#00ff00','#ffff00','#0000ff','#ff00ff','#00ffff','#ffffff'
                ];
                return base[idx];
            } else if (idx >= 16 && idx <= 231) {
                const n = idx - 16;
                const r = Math.floor(n / 36);
                const g = Math.floor((n % 36) / 6);
                const b = n % 6;
                const conv = (v) => v === 0 ? 0 : 55 + v * 40; // 0,95,135,175,215,255 approx -> 0,95.. etc; using 55+v*40 yields 55..255 and 0 for 0
                const rr = r === 0 ? 0 : 95 + (r - 1) * 40;
                const gg = g === 0 ? 0 : 95 + (g - 1) * 40;
                const bb = b === 0 ? 0 : 95 + (b - 1) * 40;
                return `#${toHex(rr)}${toHex(gg)}${toHex(bb)}`;
            } else {
                const v = 8 + 10 * (idx - 232);
                return `#${toHex(v)}${toHex(v)}${toHex(v)}`;
            }
        };

        let html = '';
        // First chunk is plain text
        html += this.escapeHtml(parts[0]).replace(/\n/g, '<br>');
        for (let i = 1; i < parts.length; i++) {
            const seg = parts[i];
            const mIndex = seg.indexOf('m');
            if (mIndex === -1) {
                // Not an SGR; keep literal '['
                html += this.escapeHtml(`[${seg}`).replace(/\n/g, '<br>');
                continue;
            }
            const seq = seg.slice(0, mIndex);
            const rest = seg.slice(mIndex + 1);
            // Parse codes like "0;31;47" and extended 38/48
            const codes = seq.length ? seq.split(';').map(c => (c === '' ? 0 : parseInt(c, 10))) : [0];
            // Close any open span before changing styles
            if (spanOpen) { html += '</span>'; spanOpen = false; }
            // Apply codes in order with lookahead for 38/48
            for (let j = 0; j < codes.length; j++) {
                const code = codes[j];
                if (isNaN(code) || code === 0) { fg = null; bg = null; bold = false; underline = false; italic = false; inverse = false; faint = false; fgIdx256 = null; }
                else if (code === 1) { bold = true; }
                else if (code === 2) { faint = true; }
                else if (code === 3) { italic = true; }
                else if (code === 4) { underline = true; }
                else if (code === 7) { inverse = true; }
                else if (code === 22) { bold = false; faint = false; }
                else if (code === 23) { italic = false; }
                else if (code === 24) { underline = false; }
                else if (code === 27) { inverse = false; }
                else if ((code >= 30 && code <= 37) || (code >= 90 && code <= 97)) { fg = colorMap[code]; if (code === 90) { fgIdx256 = 232; } }
                else if ((code >= 40 && code <= 47) || (code >= 100 && code <= 107)) { bg = bgMap[code]; }
                else if (code === 39) { fg = null; }
                else if (code === 49) { bg = null; }
                else if (code === 38 || code === 48) {
                    const isBg = (code === 48);
                    const mode = codes[j + 1];
                    if (mode === 5 && typeof codes[j + 2] !== 'undefined') {
                        const idx256 = codes[j + 2];
                        const hex = ansi256ToHex(idx256);
                        if (isBg) bg = hex; else fg = hex;
                        if (!isBg) fgIdx256 = idx256; // track fg index
                        j += 2;
                    } else if (mode === 2 && typeof codes[j + 2] !== 'undefined' && typeof codes[j + 3] !== 'undefined' && typeof codes[j + 4] !== 'undefined') {
                        const r = Math.max(0, Math.min(255, codes[j + 2]|0));
                        const g = Math.max(0, Math.min(255, codes[j + 3]|0));
                        const b = Math.max(0, Math.min(255, codes[j + 4]|0));
                        const hex = `#${toHex(r)}${toHex(g)}${toHex(b)}`;
                        if (isBg) bg = hex; else fg = hex;
                        if (!isBg) fgIdx256 = null; // truecolor, not an index; keep as not-grey
                        j += 4;
                    } else {
                        // Unrecognized extended sequence, skip
                    }
                }
            }
            // Open current style and add the rest text
            const styles = [];
            if (inverse) {
                const f = bg || '#1e1e2e';
                const b = fg || '#cdd6f4';
                styles.push(`color:${f}`);
                styles.push(`background:${b}`);
            } else {
                if (fg) styles.push(`color:${fg}`);
                if (bg) styles.push(`background:${bg}`);
            }
            if (bold) styles.push('font-weight:700');
            if (underline) styles.push('text-decoration:underline');
            if (italic) styles.push('font-style:italic');
            // Treat faint or grey 256-color (grayscale 232..251) or bright black (90) as autosuggestion -> skip
            const isGreySuggest = faint || (typeof fgIdx256 === 'number' && fgIdx256 >= 232 && fgIdx256 <= 251);
            if (!isGreySuggest) {
                if (styles.length) { html += `<span style="${styles.join(';')}">`; spanOpen = true; }
                html += this.escapeHtml(rest).replace(/\n/g, '<br>');
            }
        }
        // Close any open span at end
        if (spanOpen) html += '</span>';
        return html;
    }

    setScrollMode(mode) {
        const allowed = ['auto', 'dom', 'tmux'];
        if (allowed.includes(mode)) {
            this.tmuxScrollMode = mode;
            console.log('[WebTerminal] tmuxScrollMode =', mode);
        } else {
            console.warn('[WebTerminal] Invalid scroll mode:', mode);
        }
    }
    
    setupWheelHandler() {
        const container = document.getElementById('terminal');
        if (!container) return;

        // Clean up previous listeners
        if (this._wheelDetachFns && Array.isArray(this._wheelDetachFns)) {
            this._wheelDetachFns.forEach(fn => {
                try { fn(); } catch (_) {}
            });
        }
        this._wheelDetachFns = [];

        let wheelAccum = 0;
        let wheelFlushTimer = null;
        const flushWheel = () => {
            if (wheelAccum === 0) return;
            const direction = wheelAccum < 0 ? 'up' : 'down';
            const lineUnit = /Mac/i.test(navigator.platform) ? 50 : 40; // pixels per line heuristic
            const lines = Math.min(100, Math.max(1, Math.round(Math.abs(wheelAccum) / lineUnit)));
            wheelAccum = 0;
            if (this.currentSession && this.tmuxAvailable && this.ws && this.ws.readyState === WebSocket.OPEN) {
                this.ws.send(JSON.stringify({ type: 'TmuxScroll', direction, lines }));
                this._tmuxScrollUsed = true;
            }
        };

        const handleWheel = (e) => {
            if (this.currentSession && this.tmuxAvailable) {
                // Disable mouse scrolling entirely in tmux sessions
                e.preventDefault();
                e.stopPropagation();
                if (e.stopImmediatePropagation) e.stopImmediatePropagation();
                return false;
            }
            return true;
        };
        this.wheelHandler = handleWheel;

        // Attach in capture phase BEFORE xterm.js sees the event.
        const bind = (el) => {
            el.addEventListener('wheel', handleWheel, { passive: false, capture: true });
            // Some browsers still emit legacy events
            el.addEventListener('mousewheel', handleWheel, { passive: false, capture: true });
            el.addEventListener('DOMMouseScroll', handleWheel, { passive: false, capture: true });
            this._wheelDetachFns.push(() => {
                el.removeEventListener('wheel', handleWheel, { capture: true });
                el.removeEventListener('mousewheel', handleWheel, { capture: true });
                el.removeEventListener('DOMMouseScroll', handleWheel, { capture: true });
            });
        };

        // Bind to container immediately
        bind(container);

        // Note: We intentionally do NOT scroll the DOM in tmux mode.
        // We forward wheel to tmux copy-mode so server-side pane history is used.

        // Bind to xterm internals once available
        const attachInternals = () => {
            const viewport = container.querySelector('.xterm-viewport');
            const screen = container.querySelector('.xterm-screen');
            const xtermRoot = container.querySelector('.xterm');
            if (viewport) bind(viewport);
            if (screen) bind(screen);
            if (xtermRoot) bind(xtermRoot);
        };
        // Try now and again shortly in case xterm just mounted
        attachInternals();
        setTimeout(attachInternals, 50);
        setTimeout(attachInternals, 150);
    }
    
    initializeTerminal() {
        // Check if Terminal is loaded
        if (typeof Terminal === 'undefined') {
            console.error('Terminal not loaded. Please check xterm.js.');
            return;
        }
        
        this.terminal = new Terminal({
            cursorBlink: true,
            fontSize: 14,
            fontFamily: 'Menlo, Monaco, "Courier New", monospace',
            scrollback: 999999, // Maximum scrollback buffer to never lose history
            convertEol: true,
            // Mouse configuration: allow scrolling but disable mouse input events
            disableStdin: false, // Keep false to allow keyboard input
            // Disable all mouse tracking modes to prevent mouse events from generating input
            mouseEvents: false, // Disable mouse event tracking
            logLevel: 'off',
            theme: {
                // Zellij-inspired Catppuccin Mocha theme
                background: '#1e1e2e',
                foreground: '#cdd6f4',
                cursor: '#f5e0dc',
                cursorAccent: '#1e1e2e',
                selection: 'rgba(137, 180, 250, 0.3)',
                black: '#45475a',
                red: '#f38ba8',
                green: '#a6e3a1',
                yellow: '#f9e2af',
                blue: '#89b4fa',
                magenta: '#f5c2e7',
                cyan: '#94e2d5',
                white: '#bac2de',
                brightBlack: '#585b70',
                brightRed: '#f38ba8',
                brightGreen: '#a6e3a1',
                brightYellow: '#f9e2af',
                brightBlue: '#89b4fa',
                brightMagenta: '#f5c2e7',
                brightCyan: '#94e2d5',
                brightWhite: '#a6adc8'
            }
        });
        
        // Load FitAddon for automatic sizing
        if (typeof FitAddon !== 'undefined') {
            this.fitAddon = new FitAddon.FitAddon();
            this.terminal.loadAddon(this.fitAddon);
        }
        
        this.terminal.open(document.getElementById('terminal'));

        // Store reference for wheel handler
        this.setupWheelHandler();

        // Fit terminal to container
        if (this.fitAddon) {
            this.fitAddon.fit();
            // Fit after a short delay to ensure proper sizing
            setTimeout(() => {
                this.fitAddon.fit();
            }, 100);
        }

        // Expose terminal font size to CSS so preview can match
        try {
            const fs = (this.terminal && this.terminal.options && this.terminal.options.fontSize) ? this.terminal.options.fontSize : 14;
            document.documentElement.style.setProperty('--terminal-font-size', `${fs}px`);
        } catch (_) {}
    }
    
    setupEventListeners() {
        // Debounce resize events for better performance
        let resizeTimeout;
        window.addEventListener('resize', () => {
            clearTimeout(resizeTimeout);
            resizeTimeout = setTimeout(() => {
                if (this.fitAddon) {
                    this.fitAddon.fit();
                }
                if (this.isConnected && this.ws) {
                    this.sendResize();
                }
            }, 100);
        });
        
        window.addEventListener('keydown', (e) => {
            if ((e.ctrlKey || e.metaKey) && e.key === 'r') {
                if (!this.isConnected && (!this.ws || this.ws.readyState === WebSocket.CLOSED)) {
                    e.preventDefault();
                    this.terminal.writeln('\r\n\x1b[32mManual reconnect triggered...\x1b[0m');
                    this.reconnectAttempts = 0; // Reset attempts for manual reconnect
                    this.connect();
                }
            }
        });
        
        this.terminal.onData(data => {
            // Filter out mouse escape sequences only
            // Mouse sequences: \x1b[M or \x1b[<
            if (data.includes('\x1b[M') || data.includes('\x1b[<')) {
                console.log('Filtered mouse event');
                return;
            }
            
            if (this.isConnected && this.ws && this.ws.readyState === WebSocket.OPEN) {
                // If in view-only (tmux copy-mode), cancel it on first key press
                if (this.currentSession && this.tmuxAvailable && (this._tmuxScrollUsed || this.isScrolling)) {
                    try { this.ws.send(JSON.stringify({ type: 'TmuxCancelCopyMode' })); } catch (_) {}
                    this._tmuxScrollUsed = false;
                    this.isScrolling = false;
                    // Ensure terminal regains focus if a scroll button had it
                    const ae = document.activeElement;
                    if (ae && (ae.id === 'scroll-up' || ae.id === 'scroll-down')) {
                        try { ae.blur(); } catch(_){}
                    }
                    try { this.terminal && this.terminal.focus(); } catch(_){}
                }
                this.ws.send(JSON.stringify({
                    type: 'Input',
                    data: data
                }));
            }
        });
        
        // Only set resize handler if terminal supports it
        if (this.terminal.onResize) {
            this.terminal.onResize(({ cols, rows }) => {
                if (this.isConnected && this.ws && this.ws.readyState === WebSocket.OPEN) {
                    this.sendResize();
                }
            });
        }
        
        // Reconnect on disconnect
        window.addEventListener('beforeunload', () => {
            if (this.ws) {
                this.disconnect();
            }
        });
    }

    // Manual scroll controls: small overlay with Up/Down to enter view-only mode deliberately
    setupScrollControls() {
        // Add controls only once
        if (document.getElementById('scroll-controls')) return;
        const controls = document.createElement('div');
        controls.id = 'scroll-controls';
        controls.innerHTML = `
            <style>
                #scroll-controls { position: fixed; right: 12px; bottom: 18px; z-index: 1000; display: flex; flex-direction: column; gap: 6px; }
                #scroll-controls .btn { width: 36px; height: 36px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.2); background: rgba(49,50,68,0.9); color: #cdd6f4; cursor: pointer; font-size: 16px; display:flex; align-items:center; justify-content:center; box-shadow: 0 2px 8px rgba(0,0,0,0.3); }
                #scroll-controls .btn:hover { background: rgba(69,71,90,0.95); }
            </style>
            <button class="btn" id="scroll-up" title="Scroll up">▲</button>
            <button class="btn" id="scroll-down" title="Scroll down">▼</button>
        `;
        controls.style.display = 'none';
        document.body.appendChild(controls);

        const bumpViewOnly = () => {
            this.isScrolling = true;
            if (this.scrollIdleTimer) clearTimeout(this.scrollIdleTimer);
            this.scrollIdleTimer = setTimeout(() => {
                this.isScrolling = false;
            }, this.scrollIdleMs);
        };

        const sendScroll = (direction) => {
            if (!this.currentSession || !this.tmuxAvailable || !this.ws || this.ws.readyState !== WebSocket.OPEN) return;
            // Use a gentle default lines per click
            const lines = 5;
            this.ws.send(JSON.stringify({ type: 'TmuxScroll', direction, lines }));
            this._tmuxScrollUsed = true;
            bumpViewOnly();
            // Return focus to terminal immediately after clicking
            setTimeout(() => { try { this.terminal && this.terminal.focus(); } catch(_){} }, 0);
        };
        const btnUp = controls.querySelector('#scroll-up');
        const btnDown = controls.querySelector('#scroll-down');
        // Prevent buttons from taking focus
        btnUp.setAttribute('tabindex', '-1');
        btnDown.setAttribute('tabindex', '-1');
        btnUp.addEventListener('mousedown', (e) => { e.preventDefault(); });
        btnDown.addEventListener('mousedown', (e) => { e.preventDefault(); });
        btnUp.addEventListener('click', (e) => { e.stopPropagation(); sendScroll('up'); });
        btnDown.addEventListener('click', (e) => { e.stopPropagation(); sendScroll('down'); });
    }
    
    connect() {
        // Clean up any existing connection first
        if (this.ws) {
            // Clear heartbeat interval
            if (this.heartbeatInterval) {
                clearInterval(this.heartbeatInterval);
                this.heartbeatInterval = null;
            }
            
            // Remove event handlers to prevent memory leaks
            this.ws.onopen = null;
            this.ws.onmessage = null;
            this.ws.onerror = null;
            this.ws.onclose = null;
            
            // Force close if still open or connecting
            if (this.ws.readyState === WebSocket.CONNECTING || 
                this.ws.readyState === WebSocket.OPEN) {
                this.ws.close();
            }
            this.ws = null;
        }
        
        this.updateStatus('connecting');
        this.isHandlingDisconnect = false; // Reset flag when connecting
        
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/web-terminal/ws`;
        
        // Add timestamp to prevent caching issues
        const timestamp = new Date().getTime();
        const wsUrlWithTimestamp = `${wsUrl}?t=${timestamp}`;
        
        try {
            this.ws = new WebSocket(wsUrl); // Don't use timestamp in URL for WebSocket
        } catch (error) {
            console.error('Failed to create WebSocket:', error);
            this.terminal.writeln(`\r\n\x1b[31mFailed to create WebSocket connection: ${error.message}\x1b[0m`);
            // Try again after delay
            setTimeout(() => this.handleDisconnect(), 1000);
            return;
        }
        
        // Add connection timeout
        const connectionTimeout = setTimeout(() => {
            if (this.ws && this.ws.readyState === WebSocket.CONNECTING) {
                console.error('WebSocket connection timeout');
                this.terminal.writeln(`\r\n\x1b[33mConnection timeout, retrying...\x1b[0m`);
                this.ws.close(); // Force close the stuck connection
            }
        }, 5000); // 5 second timeout
        
        this.ws.onopen = () => {
            clearTimeout(connectionTimeout); // Clear timeout on successful connection
            // Don't clear terminal to preserve session history
            // this.terminal.clear();
            this.reconnectAttempts = 0; // Reset on successful connection
            this.reconnectDelay = 1000; // Reset delay
            this.isHandlingDisconnect = false; // Ensure flag is reset
            
            // Check if we should connect to a specific session
            if (window.targetSessionName) {
                this.ws.send(JSON.stringify({ 
                    type: 'ConnectToSession',
                    session_name: window.targetSessionName
                }));
            } else {
                this.ws.send(JSON.stringify({ type: 'Connect' }));
            }
            
            // Start heartbeat to keep connection alive (prevents Cloudflare 100s timeout)
            this.heartbeatInterval = setInterval(() => {
                if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                    this.ws.send(JSON.stringify({ type: 'Ping' }));
                }
            }, 30000); // Every 30 seconds
        };
        
        this.ws.onmessage = (event) => {
            const msg = JSON.parse(event.data);
            
            switch (msg.type) {
                case 'Connected':
                    this.isConnected = true;
                    this.currentSession = msg.session_name || null;
                    this.tmuxAvailable = msg.tmux_available || false;
                    this.updateStatus('connected');
                    this.updateSessionUI();
                    this.terminal.focus();
                    this.isScrolling = false;
                    
                    // Reset history tracking for new connection
                    if (this.currentSession) {
                        this.sessionHistory = '';
                        this.lastSavedSignature = '';
                    }
                    
                    // Re-setup wheel handler now that we know the session type
                    this.setupWheelHandler();
                    
                    // Update page title if connected to a specific session
                    if (this.currentSession) {
                        document.title = `Web Terminal - ${this.currentSession}`;
                    }
                    
                    // Fit terminal and send dimensions to backend
                    if (this.fitAddon) {
                        this.fitAddon.fit();
                    }
                    
                    setTimeout(() => {
                        this.sendResize();
                        if (this.tmuxAvailable) {
                            this.listSessions();
                        }
                    }, 50);

                    // Manage auto-save lifecycle
                    if (this.tmuxAvailable && this.currentSession) {
                        this.startAutoSave();
                    } else {
                        this.stopAutoSave();
                    }

                    // Apply any pending restore actions
                    if (this.pendingRestore && this.pendingRestore.name === this.currentSession) {
                        const { cwd, history, loadHistory } = this.pendingRestore;
                        // Change directory if provided
                        if (cwd) {
                            try {
                                this.ws.send(JSON.stringify({ type: 'Input', data: `cd '${cwd.replace(/'/g, "'\\''")}'\n` }));
                            } catch (_) {}
                        }
                        // Optionally prefill history into terminal scrollback (client-side only)
                        if (loadHistory && history) {
                            try {
                                this.terminal.writeln("\r\n\x1b[36m---- Restored history (client-side) ----\x1b[0m");
                                const chunk = history.slice(-(500000)); // cap 500k chars to avoid UI jank
                                const safeChunk = this.stripOscSequences(chunk); // preserve cursor moves/SGR; strip only OSC
                                this.terminal.write(safeChunk);
                                this.terminal.writeln("\r\n\x1b[36m---- End restored history ----\x1b[0m\r\n");
                                // Seed our history buffer so auto-save contains it
                                this.sessionHistory = safeChunk;
                                const maxLen = 5 * 1024 * 1024;
                                if (this.sessionHistory.length > maxLen) {
                                    this.sessionHistory = this.sessionHistory.slice(this.sessionHistory.length - maxLen);
                                }
                                this.lastSavedSignature = '';
                            } catch (_) {}
                        }
                        this.pendingRestore = null;
                    }
                    break;
                
                case 'SessionCreated':
                    this.handleSessionCreated(msg.session);
                    break;
                
                case 'SessionList':
                    this.handleSessionList(msg.sessions);
                    break;
                
                case 'SessionDeleted':
                    this.handleSessionDeleted(msg.name);
                    break;
                    
                case 'Output':
                    // Buffer output for batch processing to prevent blocking
                    this.outputBuffer.push(msg.data);
                    this.processOutputBuffer();
                    // Track full session history for DB persistence (bounded)
                    this.appendHistory(msg.data);
                    break;
                    
                case 'Error':
                    console.error('Terminal error:', msg.message);
                    this.terminal.writeln(`\r\n\x1b[31mError: ${msg.message}\x1b[0m`); // Red color for errors
                    if (!this.isConnected) {
                        this.updateStatus('disconnected');
                    }
                    break;
                    
                case 'Disconnected':
                    // Server-initiated disconnect
                    // Close the WebSocket, which will trigger onclose event
                    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                        this.ws.close();
                    }
                    break;
                    
                case 'Pong':
                    // Heartbeat response - connection is alive
                    console.debug('Received pong - connection alive');
                    break;
            }
        };
        
        this.ws.onerror = (error) => {
            clearTimeout(connectionTimeout); // Clear timeout on error
            console.error('WebSocket error:', error);
            // Don't call handleDisconnect here, let onclose handle it
            // This prevents double-calling when both error and close fire
        };
        
        this.ws.onclose = (event) => {
            clearTimeout(connectionTimeout); // Clear timeout on close
            
            // Clear heartbeat interval
            if (this.heartbeatInterval) {
                clearInterval(this.heartbeatInterval);
                this.heartbeatInterval = null;
            }
            
            // Log close reason for debugging
            console.log(`WebSocket closed: code=${event.code}, reason=${event.reason}, wasClean=${event.wasClean}`);
            
            // Only handle disconnect once per connection
            if (!this.isHandlingDisconnect) {
                this.handleDisconnect();
            }
        };
    }
    
    handleDisconnect() {
        // Prevent multiple simultaneous calls
        if (this.isHandlingDisconnect) {
            return;
        }
        this.isHandlingDisconnect = true;
        
        this.ws = null;
        this.isConnected = false;
        this.updateStatus('disconnected');
        this.stopAutoSave();
        this.isScrolling = false;
        
        this.reconnectAttempts++;
        
        if (this.reconnectAttempts > this.maxReconnectAttempts) {
            this.terminal.writeln('\r\n\r\nMax reconnection attempts reached.');
            this.terminal.writeln('Press \x1b[32mCtrl+R\x1b[0m to manually reconnect or refresh the page.');
            this.isHandlingDisconnect = false;
            return;
        }
        
        // Exponential backoff with cap and jitter
        // Cap at 10 seconds instead of 30 to reconnect faster when server comes back
        const baseDelay = Math.min(this.reconnectDelay * Math.pow(1.5, this.reconnectAttempts - 1), 10000);
        const jitter = Math.random() * 500; // Add up to 0.5 second of jitter
        const actualDelay = baseDelay + jitter;
        
        this.terminal.writeln(`\r\n\r\nConnection lost. Reconnecting in ${Math.ceil(actualDelay/1000)} seconds... (Attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
        
        // Auto-reconnect with backoff
        setTimeout(() => {
            if (!this.isConnected) {
                this.connect();
            }
        }, actualDelay);
    }
    
    disconnect() {
        // Clear heartbeat interval
        if (this.heartbeatInterval) {
            clearInterval(this.heartbeatInterval);
            this.heartbeatInterval = null;
        }
        
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify({ type: 'Disconnect' }));
            this.ws.close();
        }
    }

    ensureDataClient() {
        // Fast path synchronous check
        if (this.dataClient) return this.dataClient;
        try {
            const base = window.DATA_API_BASE_URL;
            if (typeof DataAPIClient !== 'undefined') {
                this.dataClient = base ? new DataAPIClient(base) : new DataAPIClient();
            } else if (typeof DataAPI !== 'undefined' && DataAPI.defaultClient) {
                this.dataClient = DataAPI.defaultClient;
            }
        } catch (_) {
            // ignore
        }
        return this.dataClient;
    }

    // Return { text, bytes } keeping last maxBytes of UTF-8
    trimUtf8Tail(str, maxBytes) {
        try {
            const enc = new TextEncoder();
            const dec = new TextDecoder();
            const buf = enc.encode(str || '');
            if (buf.length <= maxBytes) {
                return { text: str || '', bytes: buf.length };
            }
            const tail = buf.slice(buf.length - maxBytes);
            const text = dec.decode(tail);
            return { text, bytes: tail.length };
        } catch (_) {
            // Fallback using code unit length
            const s = str || '';
            const tail = s.slice(-maxBytes);
            return { text: tail, bytes: tail.length };
        }
    }

    // Strip all ANSI sequences for emptiness check only
    stripAnsiAll(str) {
        if (!str) return '';
        return str
            // OSC: ESC ] ... BEL or ST
            .replace(/\x1b\][\s\S]*?(?:\x07|\x1b\\)/g, '')
            // CSI: ESC [ ... final byte in @-~
            .replace(/\x1b\[[0-?]*[ -\/]*[@-~]/g, '')
            // Charset selection and single ESC sequences
            .replace(/\x1b[\(\)][0-9A-Za-z]/g, '')
            .replace(/\x1b[@-Z\\-_]/g, '');
    }

    // Remove empty lines (after stripping ANSI to detect visible text), keep original line (with SGR) when not empty
    removeEmptyLinesKeepAnsi(str) {
        if (!str) return '';
        const lines = str.replace(/\r/g, '').split('\n');
        const kept = [];
        for (const line of lines) {
            const visible = this.stripAnsiAll(line).trim();
            if (visible !== '') kept.push(line);
        }
        return kept.join('\n');
    }

    renderPreviewTerminal(container, raw) {
        try {
            // Dispose existing instance
            if (container._xterm && container._xterm.dispose) {
                try { container._xterm.dispose(); } catch(_){}
            }
            container.innerHTML = '';
            const fontSize = (this.terminal && this.terminal.options && this.terminal.options.fontSize) ? this.terminal.options.fontSize : 14;
            const fontFamily = (this.terminal && this.terminal.options && this.terminal.options.fontFamily) ? this.terminal.options.fontFamily : 'Menlo, Monaco, "Courier New", monospace';
            const theme = (this.terminal && this.terminal.options && this.terminal.options.theme) ? this.terminal.options.theme : undefined;
            const term = new Terminal({
                convertEol: true,
                disableStdin: true,
                cursorBlink: false,
                scrollback: 1000,
                fontSize,
                fontFamily,
                theme
            });
            let fitAddon = null;
            if (typeof FitAddon !== 'undefined') {
                fitAddon = new FitAddon.FitAddon();
                term.loadAddon(fitAddon);
            }
            container.style.height = container.style.height || '220px';
            term.open(container);
            // Avoid OSC clipboard operations for safety: strip OSC 52
            raw = raw.replace(/\x1b\]52;[\s\S]*?(?:\x07|\x1b\\)/g, '');
            term.write(raw);
            if (fitAddon) {
                setTimeout(() => { try { fitAddon.fit(); } catch(_){ } }, 0);
            }
            container._xterm = term;
        } catch (e) {
            // Fallback: plain HTML if something goes wrong
            container.innerText = raw;
        }
    }

    // Base64 helpers
    base64FromArrayBuffer(buf) {
        let binary = '';
        const bytes = new Uint8Array(buf);
        const len = bytes.length;
        for (let i = 0; i < len; i++) binary += String.fromCharCode(bytes[i]);
        return btoa(binary);
    }
    arrayBufferFromBase64(b64) {
        const binary = atob(b64);
        const len = binary.length;
        const bytes = new Uint8Array(len);
        for (let i = 0; i < len; i++) bytes[i] = binary.charCodeAt(i);
        return bytes.buffer;
    }

    async gzipToBase64(text) {
        if (typeof CompressionStream === 'undefined') return null;
        const enc = new TextEncoder();
        const cs = new CompressionStream('gzip');
        const stream = new Blob([enc.encode(text)]).stream().pipeThrough(cs);
        const ab = await new Response(stream).arrayBuffer();
        return { b64: this.base64FromArrayBuffer(ab), bytes: ab.byteLength };
    }
    async gunzipBase64ToText(b64) {
        if (typeof DecompressionStream === 'undefined') return null;
        const ab = this.arrayBufferFromBase64(b64);
        const ds = new DecompressionStream('gzip');
        const stream = new Blob([ab]).stream().pipeThrough(ds);
        const out = await new Response(stream).arrayBuffer();
        return new TextDecoder().decode(out);
    }

    async ensureDataClientAsync() {
        const existing = this.ensureDataClient();
        if (existing) return existing;
        if (this._dataApiPromise) return this._dataApiPromise;

        // Poll for presence (in case the script is still loading)
        this._dataApiPromise = new Promise((resolve) => {
            const start = Date.now();
            const check = () => {
                const c = this.ensureDataClient();
                if (c) { resolve(c); return; }
                if (Date.now() - start > 3000) { resolve(null); return; }
                setTimeout(check, 100);
            };
            check();
        });
        return this._dataApiPromise;
    }

    appendHistory(chunk) {
        if (!this.currentSession) return;
        // Ignore output generated while user is scrolling (copy-mode or viewport)
        if (this.isScrolling) return;
        try {
            this.sessionHistory += chunk;
            // Bound memory usage (~5MB)
            const maxLen = 5 * 1024 * 1024;
            if (this.sessionHistory.length > maxLen) {
                this.sessionHistory = this.sessionHistory.slice(this.sessionHistory.length - maxLen);
            }
        } catch (_) {}
    }

    async startAutoSave() {
        if (this.autoSaveInterval) return;
        const client = await this.ensureDataClientAsync();
        if (!client) return;
        this.autoSaveInterval = setInterval(() => {
            this.saveSessionToDB().catch(err => console.warn('[WebTerminal] auto-save failed:', err));
        }, this.autoSavePeriodMs);
        // Kick an initial save shortly after connect
        setTimeout(() => this.saveSessionToDB().catch(() => {}), 1000);
    }

    stopAutoSave() {
        if (this.autoSaveInterval) {
            clearInterval(this.autoSaveInterval);
            this.autoSaveInterval = null;
        }
    }

    async fetchCwd(sessionName) {
        try {
            const res = await fetch(`/web-terminal/api/sessions/${encodeURIComponent(sessionName)}/cwd`, { method: 'GET' });
            if (!res.ok) return '';
            const data = await res.json();
            return data.cwd || '';
        } catch (e) {
            return '';
        }
    }

    async saveSessionToDB() {
        if (!this.currentSession || !this.tmuxAvailable) return;
        if (this.isScrolling) return; // Skip all DB activity during scrolling
        const client = await this.ensureDataClientAsync();
        if (!client) return;

        const name = this.currentSession;
        const history = this.sessionHistory || '';
        const signature = `${history.length}:${history.slice(-200)}`;
        if (signature === this.lastSavedSignature) {
            return; // no change
        }

        const cwd = await this.fetchCwd(name);
        const nowIso = new Date().toISOString();
        let payload;

        // Upsert by name
        const where = client.buildWhereClause ? client.buildWhereClause({ name }) : `name='${name.replace(/'/g, "''")}'`;
        const results = await client.query('tmux_sessions', { where, limit: '0,1' });
        const rec = Array.isArray(results) && results.length > 0 ? results[0] : null;
        if (rec && (rec.id !== undefined || (rec.data && rec.data.id !== undefined))) {
            if (this.isScrolling) {
                // Skip update during scrolling mode
                return;
            }
            const id = rec.id ?? rec.data.id;
            // Update path: remove empty lines, cap to 100k (UTF-8 tail), then compress and store
            const noEmpty = this.removeEmptyLinesKeepAnsi(history);
            const { text: tail, bytes: unBytes } = this.trimUtf8Tail(noEmpty, this.MAX_HISTORY_BYTES);
            let encInfo = await this.gzipToBase64(tail);
            if (!encInfo) {
                // Fallback: store plain if gzip unsupported
                encInfo = { b64: null, bytes: 0 };
            }
            payload = {
                name,
                cwd,
                history_encoding: encInfo.b64 ? 'gzip+base64' : 'plain',
                history_gzip_b64: encInfo.b64 || undefined,
                history: encInfo.b64 ? undefined : tail,
                history_uncompressed_bytes: unBytes,
                history_compressed_bytes: encInfo.bytes || undefined,
                updated_at: nowIso,
                host: window.location.host
            };
            await client.update('tmux_sessions', id, payload, { override_data: true });
        } else {
            // Insert path: same as update — trim empty lines, cap to 100k, gzip+base64 if supported
            const noEmpty = this.removeEmptyLinesKeepAnsi(history);
            const { text: tail, bytes: unBytes } = this.trimUtf8Tail(noEmpty, this.MAX_HISTORY_BYTES);
            let encInfo = await this.gzipToBase64(tail);
            if (!encInfo) {
                encInfo = { b64: null, bytes: 0 };
            }
            payload = {
                name,
                cwd,
                history_encoding: encInfo.b64 ? 'gzip+base64' : 'plain',
                history_gzip_b64: encInfo.b64 || undefined,
                history: encInfo.b64 ? undefined : tail,
                history_uncompressed_bytes: unBytes,
                history_compressed_bytes: encInfo.bytes || undefined,
                updated_at: nowIso,
                host: window.location.host
            };
            await client.insert('tmux_sessions', payload);
        }
        this.lastSavedSignature = signature;
        console.debug('[WebTerminal] Session saved to DB:', name, cwd, history.length);
    }
    
    sendResize() {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            // Use fitAddon to get current dimensions if available
            if (this.fitAddon && this.fitAddon.proposeDimensions) {
                const dims = this.fitAddon.proposeDimensions();
                if (dims) {
                    this.ws.send(JSON.stringify({
                        type: 'Resize',
                        cols: dims.cols,
                        rows: dims.rows
                    }));
                }
            } else if (this.terminal) {
                // Fallback to terminal's current dimensions
                this.ws.send(JSON.stringify({
                    type: 'Resize',
                    cols: this.terminal.cols || 80,
                    rows: this.terminal.rows || 24
                }));
            }
        }
    }
    
    processOutputBuffer() {
        if (this.isProcessingBuffer || this.outputBuffer.length === 0) {
            return;
        }
        
        this.isProcessingBuffer = true;
        
        // Process buffer in chunks to avoid blocking the UI
        const processChunk = () => {
            const chunkSize = 10; // Process 10 messages at a time
            const chunk = this.outputBuffer.splice(0, chunkSize);
            
            if (chunk.length > 0) {
                // Combine chunk data and write to terminal
                const combinedData = chunk.join('');
                this.terminal.write(combinedData);
                
                // Continue processing if there's more data
                if (this.outputBuffer.length > 0) {
                    // Use setTimeout to yield control back to the event loop
                    setTimeout(processChunk, 0);
                } else {
                    this.isProcessingBuffer = false;
                }
            } else {
                this.isProcessingBuffer = false;
            }
        };
        
        processChunk();
    }
    
    updateStatus(status) {
        // Update session toggle button text to show current session
        const toggleText = document.getElementById('session-toggle-text');
        if (toggleText) {
            if (status === 'connected' && this.currentSession) {
                toggleText.textContent = this.currentSession;
            } else if (status === 'connected' && this.tmuxAvailable) {
                toggleText.textContent = 'Sessions';
            } else if (status === 'connected') {
                toggleText.textContent = 'No tmux';
            } else {
                toggleText.textContent = 'Sessions';
            }
        }
        
        // Keep minimal status handling for backward compatibility if needed
        const statusEl = document.getElementById('status');
        if (statusEl) {
            statusEl.style.display = 'none'; // Hide the status element
        }
    }
    
    setupSessionUI() {
        // Add session management controls to the page
        const sessionControls = document.createElement('div');
        sessionControls.id = 'session-controls';
        sessionControls.innerHTML = `
            <style>
                #session-controls {
                    position: fixed;
                    top: 10px;
                    right: 10px;
                    z-index: 1000;
                    font-family: 'Menlo', 'Monaco', 'Courier New', monospace;
                    font-size: 12px;
                    display: none;
                }
                
                .session-toggle {
                    background: linear-gradient(135deg, #89b4fa 0%, #cba6f7 100%);
                    color: #1e1e2e;
                    border: 1px solid rgba(137, 180, 250, 0.3);
                    padding: 8px 16px;
                    border-radius: 24px;
                    cursor: pointer;
                    display: flex;
                    align-items: center;
                    gap: 8px;
                    box-shadow: 0 4px 12px rgba(137, 180, 250, 0.2);
                    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
                    font-size: 13px;
                    font-weight: 600;
                    max-width: 200px;
                    letter-spacing: 0.3px;
                }
                
                #session-toggle-text {
                    white-space: nowrap;
                    overflow: hidden;
                    text-overflow: ellipsis;
                    max-width: 160px;
                    display: inline-block;
                }
                
                .session-toggle:hover {
                    transform: translateY(-2px);
                    box-shadow: 0 4px 15px rgba(0,0,0,0.4);
                }
                
                .session-toggle .arrow {
                    transition: transform 0.3s ease;
                    display: inline-block;
                }
                
                .session-toggle.expanded .arrow {
                    transform: rotate(180deg);
                }
                
                .session-panel {
                    position: absolute;
                    top: 100%;
                    right: 0;
                    margin-top: 8px;
                    background: rgba(30, 30, 30, 0.95);
                    backdrop-filter: blur(10px);
                    border: 1px solid rgba(255, 255, 255, 0.1);
                    border-radius: 12px;
                    padding: 0;
                    min-width: 280px;
                    max-width: 350px;
                    box-shadow: 0 8px 32px rgba(0,0,0,0.4);
                    max-height: 0;
                    overflow: hidden;
                    opacity: 0;
                    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
                }
                
                .session-panel.show {
                    max-height: 400px;
                    opacity: 1;
                    padding: 12px;
                }
                
                .session-header {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    padding-bottom: 10px;
                    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
                    margin-bottom: 10px;
                }
                
                .session-title {
                    color: #fff;
                    font-weight: 600;
                    font-size: 13px;
                }
                
                .session-actions {
                    display: flex;
                    gap: 6px;
                }
                
                .session-btn {
                    background: rgba(255, 255, 255, 0.1);
                    color: #fff;
                    border: 1px solid rgba(255, 255, 255, 0.2);
                    padding: 4px 10px;
                    border-radius: 6px;
                    cursor: pointer;
                    font-size: 11px;
                    transition: all 0.2s ease;
                }
                
                .session-btn:hover {
                    background: rgba(255, 255, 255, 0.2);
                    border-color: rgba(255, 255, 255, 0.3);
                }
                
                .session-btn.primary {
                    background: rgba(102, 126, 234, 0.8);
                    border-color: rgba(102, 126, 234, 1);
                }
                
                .session-btn.primary:hover {
                    background: rgba(102, 126, 234, 1);
                }
                
                #session-list {
                    max-height: 280px;
                    overflow-y: auto;
                    margin: 0 -4px;
                    padding: 0 4px;
                }
                
                #session-list::-webkit-scrollbar {
                    width: 6px;
                }
                
                #session-list::-webkit-scrollbar-track {
                    background: rgba(255, 255, 255, 0.05);
                    border-radius: 3px;
                }
                
                #session-list::-webkit-scrollbar-thumb {
                    background: rgba(255, 255, 255, 0.2);
                    border-radius: 3px;
                }
                
                #session-list::-webkit-scrollbar-thumb:hover {
                    background: rgba(255, 255, 255, 0.3);
                }
                
                .session-item {
                    background: rgba(255, 255, 255, 0.05);
                    border: 1px solid rgba(255, 255, 255, 0.1);
                    border-radius: 8px;
                    padding: 10px;
                    margin-bottom: 8px;
                    transition: all 0.2s ease;
                }
                
                .session-item:hover {
                    background: rgba(255, 255, 255, 0.08);
                    border-color: rgba(255, 255, 255, 0.2);
                }
                
                .session-item.current {
                    background: rgba(102, 126, 234, 0.2);
                    border-color: rgba(102, 126, 234, 0.5);
                }
                
                .session-item-header {
                    display: flex;
                    justify-content: space-between;
                    align-items: center;
                    margin-bottom: 6px;
                }
                
                .session-name {
                    color: #fff;
                    font-weight: 500;
                    font-size: 12px;
                }
                
                .session-badge {
                    background: rgba(102, 126, 234, 0.8);
                    color: white;
                    padding: 2px 6px;
                    border-radius: 4px;
                    font-size: 10px;
                    font-weight: 600;
                }
                
                .session-item-actions {
                    display: flex;
                    gap: 4px;
                }
                
                .session-item-btn {
                    background: rgba(255, 255, 255, 0.1);
                    color: #fff;
                    border: 1px solid rgba(255, 255, 255, 0.2);
                    padding: 3px 8px;
                    border-radius: 4px;
                    cursor: pointer;
                    font-size: 10px;
                    transition: all 0.2s ease;
                }
                
                .session-item-btn:hover {
                    background: rgba(255, 255, 255, 0.2);
                }
                
                .session-item-btn.connect {
                    background: rgba(76, 175, 80, 0.8);
                    border-color: rgba(76, 175, 80, 1);
                }
                
                .session-item-btn.connect:hover {
                    background: rgba(76, 175, 80, 1);
                }
                
                .session-item-btn.delete {
                    background: rgba(244, 67, 54, 0.8);
                    border-color: rgba(244, 67, 54, 1);
                }
                
                .session-item-btn.delete:hover {
                    background: rgba(244, 67, 54, 1);
                }
                
                .session-item-btn.disconnect {
                    background: rgba(255, 152, 0, 0.8);
                    border-color: rgba(255, 152, 0, 1);
                }
                
                .session-item-btn.disconnect:hover {
                    background: rgba(255, 152, 0, 1);
                }
                
                .session-item-btn.open-new {
                    background: rgba(156, 39, 176, 0.8);
                    border-color: rgba(156, 39, 176, 1);
                }
                
                .session-item-btn.open-new:hover {
                    background: rgba(156, 39, 176, 1);
                }
                
                .session-info {
                    color: rgba(255, 255, 255, 0.6);
                    font-size: 10px;
                    display: flex;
                    gap: 12px;
                }
                
                .session-empty {
                    color: rgba(255, 255, 255, 0.4);
                    text-align: center;
                    padding: 20px;
                    font-style: italic;
                }
            </style>
            
            <button class="session-toggle" id="session-toggle">
                <span id="session-toggle-text">Sessions</span>
                <span class="arrow">▼</span>
            </button>
            
            <div class="session-panel" id="session-panel">
                <div class="session-header">
                    <span class="session-title">Sessions</span>
                    <div class="session-actions">
                        <button class="session-btn" id="refresh-sessions" title="Refresh">↻</button>
                        <button class="session-btn primary" id="new-session" title="New Session">New</button>
                        <button class="session-btn" id="restore-from-db" title="Restore from DB">Restore</button>
                    </div>
                </div>
                <div id="session-list"></div>
            </div>
        `;
        document.body.appendChild(sessionControls);

        // Restore Modal (popup)
        const restoreModal = document.createElement('div');
        restoreModal.id = 'restore-modal';
        restoreModal.style.display = 'none';
        restoreModal.innerHTML = `
            <style>
                #restore-modal { position: fixed; inset: 0; z-index: 2000; }
                #restore-modal .overlay { position: absolute; inset:0; background: rgba(0,0,0,0.6); }
                #restore-modal .dialog { position: absolute; top: 10%; left: 50%; transform: translateX(-50%);
                    width: 800px; max-width: 95vw; background: rgba(30,30,46,0.98); border: 1px solid rgba(255,255,255,0.1);
                    border-radius: 12px; box-shadow: 0 8px 32px rgba(0,0,0,0.5); overflow: hidden; }
                #restore-modal .header { display:flex; align-items:center; justify-content:space-between; padding: 12px 16px; border-bottom: 1px solid rgba(255,255,255,0.1); }
                #restore-modal .header .title { font-weight: 700; color: #cdd6f4; }
                #restore-modal .body { padding: 12px 16px; }
                #restore-modal .footer { padding: 12px 16px; border-top: 1px solid rgba(255,255,255,0.1); display:flex; justify-content: flex-end; gap: 8px; }
                #saved-sessions { max-height: 50vh; overflow-y: auto; border: 1px solid rgba(255,255,255,0.08); border-radius: 8px; }
                .saved-item { padding: 10px; border-bottom: 1px solid rgba(255,255,255,0.06); display:flex; flex-direction: column; gap:8px; align-items: stretch; }
                .saved-row { display:flex; align-items:center; gap:12px; }
                .saved-name { font-weight: 600; color:#fff; }
                .saved-meta { color: rgba(255,255,255,0.6); font-size: 12px; }
                .saved-actions { margin-left: auto; display: flex; gap: 6px; }
                .saved-preview { display:none; margin-top:6px; border:1px solid rgba(255,255,255,0.1); background: rgba(0,0,0,0.25); border-radius:6px; padding:8px; font-family: Menlo, Monaco, "Courier New", monospace; font-size: var(--terminal-font-size, 14px); white-space: pre-wrap; color:#cdd6f4; max-height:220px;  }
                .btn { background: rgba(255,255,255,0.1); color: #fff; border: 1px solid rgba(255,255,255,0.2); padding: 6px 12px; border-radius: 6px; cursor: pointer; }
                .btn.primary { background: rgba(137,180,250,0.8); border-color: rgba(137,180,250,1); color: #1e1e2e; font-weight: 700; }
                .search-row { display:flex; gap:8px; margin-bottom: 10px; }
                .search-row input { flex:1; padding:8px; border-radius: 6px; border: 1px solid rgba(255,255,255,0.2); background: rgba(0,0,0,0.2); color:#fff; }
                .option-row { display:flex; align-items:center; gap:8px; margin: 8px 0 12px; color: rgba(255,255,255,0.8); }
                
            </style>
            <div class="overlay"></div>
            <div class="dialog">
                <div class="header"><div class="title">Restore Sessions from DB</div><button class="btn" id="restore-close">✕</button></div>
                <div class="body">
                    <div class="search-row">
                        <input id="restore-search" placeholder="Search by name (press Enter)" />
                        <button class="btn" id="restore-reload">Reload</button>
                    </div>
                    <div class="option-row">
                        <input type="checkbox" id="restore-load-history" checked />
                        <label for="restore-load-history">Load saved history into terminal scrollback (client-side)</label>
                    </div>
                    <div id="saved-sessions"></div>
                </div>
                <div class="footer">
                    <button class="btn" id="restore-cancel">Close</button>
                </div>
            </div>
        `;
        document.body.appendChild(restoreModal);

        const hideRestore = () => { restoreModal.style.display = 'none'; };
        const showRestore = () => { restoreModal.style.display = 'block'; this.loadSavedSessions(); };
        restoreModal.querySelector('.overlay').addEventListener('click', hideRestore);
        document.getElementById('restore-close').addEventListener('click', hideRestore);
        document.getElementById('restore-cancel').addEventListener('click', hideRestore);
        document.getElementById('restore-reload').addEventListener('click', () => this.loadSavedSessions());
        document.getElementById('restore-search').addEventListener('keydown', (e) => { if (e.key === 'Enter') this.loadSavedSessions(); });
        // inline preview handled per item

        // Toggle button functionality
        const toggleBtn = document.getElementById('session-toggle');
        const panel = document.getElementById('session-panel');
        
        toggleBtn?.addEventListener('click', () => {
            const isExpanded = toggleBtn.classList.contains('expanded');
            if (isExpanded) {
                toggleBtn.classList.remove('expanded');
                panel.classList.remove('show');
            } else {
                toggleBtn.classList.add('expanded');
                panel.classList.add('show');
                this.listSessions(); // Refresh list when opening
            }
        });
        
        // Close panel when clicking outside
        document.addEventListener('click', (e) => {
            if (!sessionControls.contains(e.target)) {
                toggleBtn?.classList.remove('expanded');
                panel?.classList.remove('show');
            }
        });
        
        // New Session Modal (same style as restore)
        const newModal = document.createElement('div');
        newModal.id = 'new-session-modal';
        newModal.style.display = 'none';
        newModal.innerHTML = `
            <style>
                #new-session-modal { position: fixed; inset: 0; z-index: 2000; }
                #new-session-modal .overlay { position: absolute; inset:0; background: rgba(0,0,0,0.6); }
                #new-session-modal .dialog { position: absolute; top: 20%; left: 50%; transform: translateX(-50%);
                    width: 520px; max-width: 92vw; background: rgba(30,30,46,0.98); border: 1px solid rgba(255,255,255,0.1);
                    border-radius: 12px; box-shadow: 0 8px 32px rgba(0,0,0,0.5); overflow: hidden; }
                #new-session-modal .header { display:flex; align-items:center; justify-content:space-between; padding: 12px 16px; border-bottom: 1px solid rgba(255,255,255,0.1); color:#cdd6f4; }
                #new-session-modal .title { font-weight: 700; }
                #new-session-modal .body { padding: 12px 16px; }
                #new-session-modal .footer { padding: 12px 16px; border-top: 1px solid rgba(255,255,255,0.1); display:flex; justify-content: flex-end; gap: 8px; }
                #new-session-name { width: 100%; padding: 10px; border-radius: 8px; border: 1px solid rgba(255,255,255,0.2); background: rgba(0,0,0,0.2); color:#fff; font-family: Menlo, Monaco, "Courier New", monospace; }
                .hint { color: rgba(255,255,255,0.6); font-size:12px; margin-top: 8px; }
                .btn { background: rgba(255,255,255,0.1); color: #fff; border: 1px solid rgba(255,255,255,0.2); padding: 6px 12px; border-radius: 6px; cursor: pointer; }
                .btn.primary { background: rgba(137,180,250,0.8); border-color: rgba(137,180,250,1); color: #1e1e2e; font-weight: 700; }
            </style>
            <div class="overlay"></div>
            <div class="dialog">
                <div class="header"><div class="title">Create New Session</div><button class="btn" id="new-close">✕</button></div>
                <div class="body">
                    <input id="new-session-name" placeholder="Enter session name (optional)" />
                    <div class="hint">Leave empty to auto-generate a name.</div>
                </div>
                <div class="footer">
                    <button class="btn" id="new-cancel">Cancel</button>
                    <button class="btn primary" id="new-create">Create</button>
                </div>
            </div>
        `;
        document.body.appendChild(newModal);

        const hideNew = () => { newModal.style.display = 'none'; };
        const showNew = () => { newModal.style.display = 'block'; setTimeout(() => { document.getElementById('new-session-name')?.focus(); }, 0); };
        newModal.querySelector('.overlay').addEventListener('click', hideNew);
        document.getElementById('new-close').addEventListener('click', hideNew);
        document.getElementById('new-cancel').addEventListener('click', hideNew);
        document.getElementById('new-create').addEventListener('click', () => {
            const val = (document.getElementById('new-session-name').value || '').trim();
            hideNew();
            this.createSession(val);
        });
        document.getElementById('new-session-name').addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                document.getElementById('new-create').click();
            }
        });

        // Delete Session Modal (same style as restore)
        const delModal = document.createElement('div');
        delModal.id = 'delete-modal';
        delModal.style.display = 'none';
        delModal.innerHTML = `
            <style>
                #delete-modal { position: fixed; inset: 0; z-index: 2000; }
                #delete-modal .overlay { position: absolute; inset:0; background: rgba(0,0,0,0.6); }
                #delete-modal .dialog { position: absolute; top: 25%; left: 50%; transform: translateX(-50%);
                    width: 520px; max-width: 92vw; background: rgba(30,30,46,0.98); border: 1px solid rgba(255,255,255,0.1);
                    border-radius: 12px; box-shadow: 0 8px 32px rgba(0,0,0,0.5); overflow: hidden; }
                #delete-modal .header { display:flex; align-items:center; justify-content:space-between; padding: 12px 16px; border-bottom: 1px solid rgba(255,255,255,0.1); color:#cdd6f4; }
                #delete-modal .title { font-weight: 700; }
                #delete-modal .body { padding: 12px 16px; color: #cdd6f4; }
                #delete-modal .footer { padding: 12px 16px; border-top: 1px solid rgba(255,255,255,0.1); display:flex; justify-content: flex-end; gap: 8px; }
                .danger { background: rgba(244, 67, 54, 0.85); border: 1px solid rgba(244,67,54, 1); color: white; }
                .btn { background: rgba(255,255,255,0.1); color: #fff; border: 1px solid rgba(255,255,255,0.2); padding: 6px 12px; border-radius: 6px; cursor: pointer; }
                .code { background: rgba(0,0,0,0.3); padding:2px 6px; border-radius: 6px; font-family: Menlo, Monaco, "Courier New", monospace; }
            </style>
            <div class="overlay"></div>
            <div class="dialog">
                <div class="header"><div class="title">Delete Session</div><button class="btn" id="del-close">✕</button></div>
                <div class="body">Are you sure you want to delete session <span class="code" id="del-name"></span>?</div>
                <div class="footer">
                    <button class="btn" id="del-cancel">Cancel</button>
                    <button class="btn danger" id="del-confirm">Delete</button>
                </div>
            </div>
        `;
        document.body.appendChild(delModal);
        const hideDel = () => { delModal.style.display = 'none'; };
        const showDel = (name) => {
            delModal.style.display = 'block';
            const el = document.getElementById('del-name'); if (el) el.textContent = name;
            const btn = document.getElementById('del-confirm'); if (btn) btn.dataset.session = name;
        };
        delModal.querySelector('.overlay').addEventListener('click', hideDel);
        document.getElementById('del-close').addEventListener('click', hideDel);
        document.getElementById('del-cancel').addEventListener('click', hideDel);
        document.getElementById('del-confirm').addEventListener('click', (e) => {
            const name = e.target.dataset.session;
            hideDel();
            this.deleteSession(name, { skipConfirm: true });
        });

        // Add event listeners for session controls
        document.getElementById('refresh-sessions')?.addEventListener('click', (e) => {
            e.stopPropagation();
            this.listSessions();
        });
        
        document.getElementById('new-session')?.addEventListener('click', (e) => {
            e.stopPropagation();
            showNew();
        });
        document.getElementById('restore-from-db')?.addEventListener('click', (e) => {
            e.stopPropagation();
            showRestore();
        });
        
        // Use event delegation for dynamically created buttons
        document.getElementById('session-list')?.addEventListener('click', (e) => {
            e.stopPropagation();
            if (e.target.tagName === 'BUTTON') {
                const action = e.target.dataset.action;
                const sessionName = e.target.dataset.session;
                
                if (action === 'connect') {
                    this.connectToSession(sessionName);
                    // Close panel after connecting
                    toggleBtn?.classList.remove('expanded');
                    panel?.classList.remove('show');
                } else if (action === 'disconnect') {
                    this.disconnectFromSession();
                    // Close panel after disconnecting
                    toggleBtn?.classList.remove('expanded');
                    panel?.classList.remove('show');
                } else if (action === 'delete') {
                    showDel(sessionName);
                } else if (action === 'open-new') {
                    // Open session in new tab/window
                    window.open(`/web-terminal/${sessionName}`, '_blank');
                }
            }
        });
    }
    
    updateSessionUI() {
        const controls = document.getElementById('session-controls');
        if (controls) {
            controls.style.display = this.tmuxAvailable ? 'block' : 'none';
        }
        const scrollCtrls = document.getElementById('scroll-controls');
        if (scrollCtrls) {
            // Show only for tmux sessions (not local terminal)
            const show = !!(this.tmuxAvailable && this.currentSession);
            scrollCtrls.style.display = show ? 'flex' : 'none';
        }
    }
    
    listSessions() {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify({ type: 'ListSessions' }));
        }
    }
    
    createSession(name) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify({ 
                type: 'CreateSession',
                name: name || null
            }));
        }
    }
    
    connectToSession(sessionName) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify({ 
                type: 'ConnectToSession',
                session_name: sessionName
            }));
        }
    }
    
    deleteSession(sessionName, opts = {}) {
        if (!sessionName) return;
        const { skipConfirm = false } = opts;
        if (!skipConfirm) {
            // Fallback safety: if used directly, require confirm
            if (!confirm(`Delete session '${sessionName}'?`)) return;
        }
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify({ 
                type: 'DeleteSession',
                name: sessionName
            }));
        }
    }
    
    disconnectFromSession() {
        if (this.currentSession) {
            const sessionName = this.currentSession;
            this.currentSession = null;
            this.stopAutoSave();
            this.sessionHistory = '';
            if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                // Connect to a regular terminal (no session)
                this.ws.send(JSON.stringify({ type: 'Connect' }));
                this.terminal.writeln(`\r\n\x1b[33mDisconnected from tmux session: ${sessionName}\x1b[0m`);
                this.updateStatus('connected'); // Update button text
            }
        }
    }
    
    handleSessionCreated(session) {
        this.terminal.writeln(`\r\n\x1b[32mSession created: ${session.name}\x1b[0m`);
        this.listSessions();
        // Auto-connect to the new session
        this.connectToSession(session.name);
    }
    
    handleSessionList(sessions) {
        const listElement = document.getElementById('session-list');
        if (!listElement) return;
        
        if (sessions.length === 0) {
            listElement.innerHTML = '<div class="session-empty">No active sessions</div>';
            return;
        }
        
        listElement.innerHTML = sessions.map(session => {
            const isCurrent = this.currentSession === session.name;
            return `
                <div class="session-item ${isCurrent ? 'current' : ''}">
                    <div class="session-item-header">
                        <span class="session-name">${session.name}</span>
                        ${isCurrent ? '<span class="session-badge">CURRENT</span>' : ''}
                    </div>
                    <div class="session-item-actions">
                        ${!isCurrent ? 
                            `<button class="session-item-btn connect" data-action="connect" data-session="${session.name}">Connect</button>` 
                            : `<button class="session-item-btn disconnect" data-action="disconnect" data-session="${session.name}">Disconnect</button>`}
                        <button class="session-item-btn open-new" data-action="open-new" data-session="${session.name}">Open New</button>
                        <button class="session-item-btn delete" data-action="delete" data-session="${session.name}">Delete</button>
                    </div>
                    <div class="session-info">
                        <span>Windows: ${session.window_count}</span>
                        <span>Clients: ${session.attached_clients}</span>
                    </div>
                </div>
            `;
        }).join('');
    }
    
    handleSessionDeleted(name) {
        this.terminal.writeln(`\r\n\x1b[33mSession deleted: ${name}\x1b[0m`);
        if (this.currentSession === name) {
            this.currentSession = null;
            this.stopAutoSave();
            this.sessionHistory = '';
            // Reconnect to a regular terminal
            this.ws.send(JSON.stringify({ type: 'Connect' }));
            this.updateStatus('connected'); // Update button text
        }
        this.listSessions();
    }

    async loadSavedSessions() {
        const listEl = document.getElementById('saved-sessions');
        const client = await this.ensureDataClientAsync();
        if (!client) {
            listEl.innerHTML = '<div class="session-empty">Data API not available</div>';
            return;
        }
        const search = (document.getElementById('restore-search')?.value || '').trim();
        let where = '';
        if (search) {
            // Basic contains: name LIKE '%search%'
            const esc = search.replace(/'/g, "''");
            where = `name LIKE '%${esc}%'`;
        }
        let entries = [];
        try {
            entries = await client.query('tmux_sessions', { where, order_by: 'updated desc', limit: '0,50' });
        } catch (e) {
            listEl.innerHTML = `<div class="session-empty">Failed to load: ${e.message}</div>`;
            return;
        }
        if (!Array.isArray(entries) || entries.length === 0) {
            listEl.innerHTML = '<div class="session-empty">No saved sessions</div>';
            return;
        }
        const rows = entries.map((rec) => {
            const data = rec.data || rec;
            const name = data.name || '(unknown)';
            const cwd = data.cwd || '';
            const updated = rec.updated || data.updated_at || '';
            const bytes = data.history_uncompressed_bytes || data.history_bytes || (data.history ? data.history.length : 0);
            const id = rec.id || data.id || '';
            return { id, name, cwd, updated, bytes, history: data.history || '', history_gzip_b64: data.history_gzip_b64, history_encoding: data.history_encoding };
        });
        listEl.innerHTML = rows.map((r, idx) => (
            `<div class="saved-item" data-index="${idx}">
                <div class="saved-row">
                    <div>
                        <div class="saved-name">${r.name}</div>
                        <div class="saved-meta">cwd: ${r.cwd || 'n/a'} — updated: ${r.updated || ''} — history: ${r.bytes} bytes</div>
                    </div>
                    <div class="saved-actions">
                        <button class="btn" data-action="preview" data-index="${idx}">Preview</button>
                        <button class="btn primary" data-action="restore" data-index="${idx}">Restore</button>
                    </div>
                </div>
                <div class="saved-preview" id="saved-preview-${idx}"></div>
            </div>`
        )).join('');

        listEl.onclick = async (e) => {
            const btn = e.target.closest('button');
            if (!btn) return;
            const idx = parseInt(btn.getAttribute('data-index'), 10);
            if (isNaN(idx)) return;
            const items = listEl.querySelectorAll('.saved-item');
            const chosen = rows[idx];
            if (!chosen) return;
            if (btn.getAttribute('data-action') === 'preview') {
                const itemEl = btn.closest('.saved-item');
                const prevEl = itemEl.querySelector('.saved-preview');
                if (!prevEl) return;
                if (prevEl.style.display === 'block') {
                    prevEl.style.display = 'none';
                    btn.textContent = 'Preview';
                } else {
                    let raw = (chosen.history || '');
                    if ((!raw || raw.length === 0) && chosen.history_gzip_b64 && chosen.history_encoding === 'gzip+base64') {
                        try { raw = await this.gunzipBase64ToText(chosen.history_gzip_b64); } catch (_) {}
                    }
                    // raw = (raw || '').slice(-20000);
                    // // Render with an offscreen xterm instance for exact fidelity
                    this.renderPreviewTerminal(prevEl, raw);
                    prevEl.style.display = 'block';
                    btn.textContent = 'Hide';
                }
            } else if (btn.getAttribute('data-action') === 'restore') {
                const loadHistory = !!document.getElementById('restore-load-history')?.checked;
                // Ensure we have decompressed history if needed
                if ((!chosen.history || chosen.history.length === 0) && chosen.history_gzip_b64 && chosen.history_encoding === 'gzip+base64') {
                    try { chosen.history = await this.gunzipBase64ToText(chosen.history_gzip_b64); } catch (_) {}
                }
                this.restoreFromDB(chosen, { loadHistory });
                // Close dialog
                const modal = document.getElementById('restore-modal');
                if (modal) modal.style.display = 'none';
            }
        };
    }

    async restoreFromDB(entry, options = {}) {
        const { name, cwd, history } = entry;
        const { loadHistory } = options;
        if (!this.tmuxAvailable) {
            this.terminal.writeln('\r\n\x1b[31mCannot restore: tmux not available\x1b[0m');
            return;
        }
        // Try to connect if exists; otherwise create
        const connectAfter = () => this.connectToSession(name);
        // Use cached session list if available by requesting fresh list and checking after a tick
        let exists = false;
        const listCheck = new Promise((resolve) => {
            const handler = (ev) => {
                try {
                    const msg = JSON.parse(ev.data);
                    if (msg.type === 'SessionList') {
                        exists = (msg.sessions || []).some(s => s.name === name);
                        resolve(undefined);
                        this.ws.removeEventListener('message', handler);
                    }
                } catch (_) {}
            };
            this.ws.addEventListener('message', handler);
            this.listSessions();
            setTimeout(() => { try { this.ws.removeEventListener('message', handler); } catch(_){} resolve(undefined); }, 300);
        });
        await listCheck;
        if (exists) {
            connectAfter();
        } else {
            this.createSession(name);
            // connect will happen via SessionCreated handler
        }
        // Prepare post-connect actions
        this.pendingRestore = { name, cwd, history, loadHistory };
    }
}

// Wait for all scripts to load
// Global variable to hold the terminal instance
let webTerminal = null;

function initTerminal() {
    // Only check for Terminal since addons are optional in 5.5.0
    if (typeof Terminal === 'undefined') {
        console.log('Waiting for Terminal to load...');
        setTimeout(initTerminal, 100);
        return;
    }
    console.log('Terminal loaded, initializing...');
    // Create and store globally
    webTerminal = new WebTerminal();
    // Also make it available on window for onclick handlers
    window.webTerminal = webTerminal;
}

// Use window.onload to ensure all scripts are loaded
window.addEventListener('load', () => {
    initTerminal();
});
