class WebTerminal {
    constructor() {
        this.terminal = null;
        this.fitAddon = null;
        this.ws = null;
        this.isConnected = false;
        
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
        
        // For xterm 5.5.0 - addons might not be available
        if (typeof FitAddon !== 'undefined') {
            this.fitAddon = new FitAddon();
            this.terminal.loadAddon(this.fitAddon);
        }
        
        this.terminal.open(document.getElementById('terminal'));
        
        // Only call fit if fitAddon is available
        if (this.fitAddon) {
            this.fitAddon.fit();
        }
    }
    
    setupEventListeners() {
        window.addEventListener('resize', () => {
            if (this.fitAddon && this.fitAddon.fit) {
                this.fitAddon.fit();
                if (this.isConnected && this.ws) {
                    this.sendResize();
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
        this.updateStatus('connecting');
        
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${protocol}//${window.location.host}/web-terminal/ws`;
        
        this.ws = new WebSocket(wsUrl);
        
        this.ws.onopen = () => {
            this.terminal.clear();
            this.ws.send(JSON.stringify({ type: 'Connect' }));
        };
        
        this.ws.onmessage = (event) => {
            const msg = JSON.parse(event.data);
            
            switch (msg.type) {
                case 'Connected':
                    this.isConnected = true;
                    this.updateStatus('connected');
                    this.terminal.focus();
                    // Initial resize
                    this.sendResize();
                    break;
                    
                case 'Output':
                    this.terminal.write(msg.data);
                    break;
                    
                case 'Error':
                    console.error('Terminal error:', msg.message);
                    if (!this.isConnected) {
                        this.updateStatus('disconnected');
                        this.terminal.writeln(`\r\nError: ${msg.message}`);
                    }
                    break;
                    
                case 'Disconnected':
                    this.handleDisconnect();
                    break;
            }
        };
        
        this.ws.onerror = (error) => {
            console.error('WebSocket error:', error);
            this.handleDisconnect();
        };
        
        this.ws.onclose = () => {
            this.handleDisconnect();
        };
    }
    
    handleDisconnect() {
        this.ws = null;
        this.isConnected = false;
        this.updateStatus('disconnected');
        
        this.terminal.writeln('\r\n\r\nConnection lost. Reconnecting in 3 seconds...');
        
        // Auto-reconnect after 3 seconds
        setTimeout(() => {
            if (!this.isConnected) {
                this.connect();
            }
        }, 3000);
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