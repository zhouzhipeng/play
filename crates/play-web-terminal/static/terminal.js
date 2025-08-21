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
        
        this.initializeTerminal();
        this.setupEventListeners();
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
        this.updateStatus('connecting');
        this.isHandlingDisconnect = false; // Reset flag when connecting
        
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/web-terminal/ws`;
        
        this.ws = new WebSocket(wsUrl);
        
        this.ws.onopen = () => {
            this.terminal.clear();
            this.reconnectAttempts = 0; // Reset on successful connection
            this.reconnectDelay = 1000; // Reset delay
            this.isHandlingDisconnect = false; // Ensure flag is reset
            this.ws.send(JSON.stringify({ type: 'Connect' }));
        };
        
        this.ws.onmessage = (event) => {
            const msg = JSON.parse(event.data);
            
            switch (msg.type) {
                case 'Connected':
                    this.isConnected = true;
                    this.updateStatus('connected');
                    this.terminal.focus();
                    // Ensure proper sizing after connection
                    if (this.fitAddon) {
                        this.fitAddon.fit();
                    }
                    // Send resize after fitting
                    setTimeout(() => {
                        this.sendResize();
                    }, 50);
                    break;
                    
                case 'Output':
                    this.terminal.write(msg.data);
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
            }
        };
        
        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            // Don't call handleDisconnect here, let onclose handle it
            // This prevents double-calling when both error and close fire
        };
        
        this.ws.onclose = () => {
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
            this.terminal.writeln('\r\n\r\nMax reconnection attempts reached. Please refresh the page to try again.');
            this.isHandlingDisconnect = false;
            return;
        }
        
        // Exponential backoff with jitter
        const delay = Math.min(this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1), 30000);
        const jitter = Math.random() * 1000; // Add up to 1 second of jitter
        const actualDelay = delay + jitter;
        
        this.terminal.writeln(`\r\n\r\nConnection lost. Reconnecting in ${Math.ceil(actualDelay/1000)} seconds... (Attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);
        
        // Auto-reconnect with backoff
        setTimeout(() => {
            if (!this.isConnected) {
                this.connect();
            }
        }, actualDelay);
    }
    
    disconnect() {
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
    
    updateStatus(status) {
        const statusEl = document.getElementById('status');
        statusEl.className = 'status ' + status;
        
        switch (status) {
            case 'connected':
                statusEl.textContent = 'Connected';
                break;
            case 'connecting':
                statusEl.textContent = 'Connecting...';
                break;
            case 'disconnected':
                statusEl.textContent = 'Disconnected';
                break;
        }
    }
}

// Wait for all scripts to load
function initTerminal() {
    // Only check for Terminal since addons are optional in 5.5.0
    if (typeof Terminal === 'undefined') {
        console.log('Waiting for Terminal to load...');
        setTimeout(initTerminal, 100);
        return;
    }
    console.log('Terminal loaded, initializing...');
    new WebTerminal();
}

// Use window.onload to ensure all scripts are loaded
window.addEventListener('load', () => {
    initTerminal();
});