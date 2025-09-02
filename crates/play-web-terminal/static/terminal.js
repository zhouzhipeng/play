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
            this.ws.send(JSON.stringify({ type: 'Connect' }));
            
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
        const statusEl = document.getElementById('status');
        statusEl.className = 'status ' + status;
        
        switch (status) {
            case 'connected':
                let statusText = 'Connected';
                if (this.currentSession) {
                    statusText += ` (session: ${this.currentSession})`;
                }
                statusEl.textContent = statusText;
                break;
            case 'connecting':
                statusEl.textContent = 'Connecting...';
                break;
            case 'disconnected':
                statusEl.textContent = 'Disconnected';
                break;
        }
    }
    
    setupSessionUI() {
        // Add session management controls to the page
        const sessionControls = document.createElement('div');
        sessionControls.id = 'session-controls';
        sessionControls.style.cssText = 'position: absolute; top: 10px; right: 10px; z-index: 1000; background: #1e1e1e; padding: 10px; border-radius: 5px; display: none;';
        sessionControls.innerHTML = `
            <div style="color: #cccccc; margin-bottom: 10px;">
                <strong>tmux Sessions</strong>
                <button id="refresh-sessions" style="margin-left: 10px; padding: 2px 8px;">Refresh</button>
                <button id="new-session" style="margin-left: 5px; padding: 2px 8px;">New</button>
            </div>
            <div id="session-list" style="max-height: 200px; overflow-y: auto;"></div>
        `;
        document.body.appendChild(sessionControls);
        
        // Add event listeners for session controls
        document.getElementById('refresh-sessions')?.addEventListener('click', () => {
            this.listSessions();
        });
        
        document.getElementById('new-session')?.addEventListener('click', () => {
            const name = prompt('Enter session name (leave empty for auto-generated):');
            this.createSession(name);
        });
        
        // Use event delegation for dynamically created buttons
        document.getElementById('session-list')?.addEventListener('click', (e) => {
            if (e.target.tagName === 'BUTTON') {
                const action = e.target.dataset.action;
                const sessionName = e.target.dataset.session;
                
                if (action === 'connect') {
                    this.connectToSession(sessionName);
                } else if (action === 'delete') {
                    this.deleteSession(sessionName);
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
            listElement.innerHTML = '<div style="color: #666;">No sessions</div>';
            return;
        }
        
        listElement.innerHTML = sessions.map(session => `
            <div style="color: #cccccc; padding: 5px; border-bottom: 1px solid #333;">
                <div style="display: flex; justify-content: space-between; align-items: center;">
                    <span>${session.name} ${this.currentSession === session.name ? '(current)' : ''}</span>
                    <div>
                        ${this.currentSession !== session.name ? 
                            `<button data-action="connect" data-session="${session.name}" style="padding: 2px 6px; margin-left: 5px;">Connect</button>` : ''}
                        <button data-action="delete" data-session="${session.name}" style="padding: 2px 6px; margin-left: 5px;">Delete</button>
                    </div>
                </div>
                <div style="font-size: 11px; color: #666;">
                    Windows: ${session.window_count} | Clients: ${session.attached_clients}
                </div>
            </div>
        `).join('');
    }
    
    handleSessionDeleted(name) {
        this.terminal.writeln(`\r\n\x1b[33mSession deleted: ${name}\x1b[0m`);
        if (this.currentSession === name) {
            this.currentSession = null;
            // Reconnect to a regular terminal
            this.ws.send(JSON.stringify({ type: 'Connect' }));
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