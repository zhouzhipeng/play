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
        
        this.initializeTerminal();
        this.setupEventListeners();
        this.setupSessionUI();
        this.connect(); // Auto-connect on load
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
            scrollback: 1000,
            // Disable all mouse events from being sent to the terminal
            mouseEvents: false,
            // Disable application cursor mode
            applicationCursor: false,
            // Disable alternate screen buffer (used by vim, less, etc)
            alternateScroll: false,
            theme: {
                background: '#1e1e1e',
                foreground: '#cccccc',
                cursor: '#ffffff',
                selection: '#264f78',
                black: '#000000',
                red: '#cd3131',
                green: '#0dbc79',
                yellow: '#e5e510',
                blue: '#2472c8',
                magenta: '#bc3fbc',
                cyan: '#11a8cd',
                white: '#e5e5e5',
                brightBlack: '#666666',
                brightRed: '#f14c4c',
                brightGreen: '#23d18b',
                brightYellow: '#f5f543',
                brightBlue: '#3b8eea',
                brightMagenta: '#d670d6',
                brightCyan: '#29b8db',
                brightWhite: '#e5e5e5'
            }
        });
        
        // Load FitAddon for automatic sizing
        if (typeof FitAddon !== 'undefined') {
            this.fitAddon = new FitAddon.FitAddon();
            this.terminal.loadAddon(this.fitAddon);
        }
        
        this.terminal.open(document.getElementById('terminal'));
        
        // Completely override mouse wheel behavior
        const terminalElement = document.getElementById('terminal');
        
        // Capture wheel events at multiple levels
        const preventWheel = (e) => {
            // Stop the event completely
            e.preventDefault();
            e.stopPropagation();
            e.stopImmediatePropagation();
            
            // Check if horizontal scrolling (deltaX) is significant
            // This prevents browser back/forward navigation on horizontal swipe
            if (Math.abs(e.deltaX) > Math.abs(e.deltaY)) {
                // Just prevent the default behavior for horizontal scrolling
                // Don't scroll the terminal for horizontal movements
                return false;
            }
            
            // Implement our own scrolling for the terminal buffer (vertical only)
            const scrollAmount = e.deltaY > 0 ? 3 : -3;
            this.terminal.scrollLines(scrollAmount);
            
            return false;
        };
        
        // Add listeners at capture phase to intercept early
        terminalElement.addEventListener('wheel', preventWheel, { capture: true, passive: false });
        terminalElement.addEventListener('mousewheel', preventWheel, { capture: true, passive: false });
        terminalElement.addEventListener('DOMMouseScroll', preventWheel, { capture: true, passive: false });
        
        // Also prevent on the terminal's viewport element if it exists
        setTimeout(() => {
            const viewport = terminalElement.querySelector('.xterm-viewport');
            if (viewport) {
                viewport.addEventListener('wheel', preventWheel, { capture: true, passive: false });
                viewport.addEventListener('mousewheel', preventWheel, { capture: true, passive: false });
                viewport.addEventListener('DOMMouseScroll', preventWheel, { capture: true, passive: false });
            }
            
            // Also on the screen element
            const screen = terminalElement.querySelector('.xterm-screen');
            if (screen) {
                screen.addEventListener('wheel', preventWheel, { capture: true, passive: false });
                screen.addEventListener('mousewheel', preventWheel, { capture: true, passive: false });
                screen.addEventListener('DOMMouseScroll', preventWheel, { capture: true, passive: false });
            }
        }, 100);
        
        // Fit terminal to container
        if (this.fitAddon) {
            // Initial fit
            this.fitAddon.fit();
            // Fit after a short delay to ensure proper sizing
            setTimeout(() => {
                this.fitAddon.fit();
            }, 100);
        }
    }
    
    setupEventListeners() {
        // Debounce resize events for better performance
        let resizeTimeout;
        window.addEventListener('resize', () => {
            clearTimeout(resizeTimeout);
            resizeTimeout = setTimeout(() => {
                if (this.fitAddon && this.fitAddon.fit) {
                    this.fitAddon.fit();
                    if (this.isConnected && this.ws) {
                        this.sendResize();
                    }
                }
            }, 100);
        });
        
        // Add keyboard shortcut for manual reconnect (Ctrl+R or Cmd+R)
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
            // Filter out mouse-related escape sequences
            // Mouse sequences typically start with \x1b[M or \x1b[< 
            if (data.includes('\x1b[M') || data.includes('\x1b[<')) {
                console.log('Filtered mouse escape sequence');
                return;
            }
            
            if (this.isConnected && this.ws && this.ws.readyState === WebSocket.OPEN) {
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
            this.terminal.clear();
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
                    
                    // Update page title if connected to a specific session
                    if (this.currentSession) {
                        document.title = `Web Terminal - ${this.currentSession}`;
                    }
                    
                    // Ensure proper sizing after connection
                    if (this.fitAddon) {
                        this.fitAddon.fit();
                    }
                    // Send resize after fitting
                    setTimeout(() => {
                        this.sendResize();
                        if (this.tmuxAvailable) {
                            this.listSessions();
                        }
                    }, 50);
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
    
    sendResize() {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            // If fitAddon is available, use it; otherwise use terminal dimensions
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
                // Fallback to terminal's cols/rows
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
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    color: white;
                    border: none;
                    padding: 8px 12px;
                    border-radius: 20px;
                    cursor: pointer;
                    display: flex;
                    align-items: center;
                    gap: 6px;
                    box-shadow: 0 2px 10px rgba(0,0,0,0.3);
                    transition: all 0.3s ease;
                    font-size: 12px;
                    font-weight: 600;
                    max-width: 200px;
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
                    <span class="session-title">tmux Sessions</span>
                    <div class="session-actions">
                        <button class="session-btn" id="refresh-sessions" title="Refresh">↻</button>
                        <button class="session-btn primary" id="new-session" title="New Session">+ New</button>
                    </div>
                </div>
                <div id="session-list"></div>
            </div>
        `;
        document.body.appendChild(sessionControls);
        
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
        
        // Add event listeners for session controls
        document.getElementById('refresh-sessions')?.addEventListener('click', (e) => {
            e.stopPropagation();
            this.listSessions();
        });
        
        document.getElementById('new-session')?.addEventListener('click', (e) => {
            e.stopPropagation();
            const name = prompt('Enter session name (leave empty for auto-generated):');
            if (name !== null) {
                this.createSession(name);
            }
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
                    this.deleteSession(sessionName);
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
    
    deleteSession(sessionName) {
        if (confirm(`Delete session '${sessionName}'?`)) {
            if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                this.ws.send(JSON.stringify({ 
                    type: 'DeleteSession',
                    name: sessionName
                }));
            }
        }
    }
    
    disconnectFromSession() {
        if (this.currentSession) {
            const sessionName = this.currentSession;
            this.currentSession = null;
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
            // Reconnect to a regular terminal
            this.ws.send(JSON.stringify({ type: 'Connect' }));
            this.updateStatus('connected'); // Update button text
        }
        this.listSessions();
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