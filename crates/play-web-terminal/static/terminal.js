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