/**
 * Enhanced scrollback buffer management for smooth Zellij-like scrolling
 * This replaces tmux copy-mode with client-side scrollback handling
 */
class ScrollbackManager {
    constructor(terminal) {
        this.terminal = terminal;
        this.scrollbackBuffer = [];
        this.maxBufferSize = 100000; // Maximum lines to keep in memory
        this.viewportOffset = 0; // Current scroll position (0 = bottom/latest)
        this.isScrolling = false;
        this.bufferUpdatePending = false;
        
        // Track terminal dimensions
        this.cols = terminal.cols || 80;
        this.rows = terminal.rows || 24;
        
        // Create overlay for smooth scrolling
        this.createScrollOverlay();
        
        // Bind to terminal data events to capture all output
        this.captureTerminalOutput();
    }
    
    createScrollOverlay() {
        // Create a hidden terminal instance for scrollback display
        this.scrollTerminal = new Terminal({
            cols: this.cols,
            rows: this.rows,
            scrollback: 0, // We manage scrollback ourselves
            disableStdin: true,
            theme: this.terminal.options.theme,
            fontSize: this.terminal.options.fontSize,
            fontFamily: this.terminal.options.fontFamily,
            allowTransparency: false,
            convertEol: true
        });
        
        // Create overlay container
        this.overlay = document.createElement('div');
        this.overlay.id = 'scrollback-overlay';
        this.overlay.style.cssText = `
            position: absolute;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            z-index: 10;
            display: none;
            background: ${this.terminal.options.theme?.background || '#1e1e2e'};
        `;
        
        const terminalContainer = document.getElementById('terminal');
        if (terminalContainer) {
            terminalContainer.appendChild(this.overlay);
            this.scrollTerminal.open(this.overlay);
        }
    }
    
    captureTerminalOutput() {
        // Intercept terminal write operations to build scrollback buffer
        const originalWrite = this.terminal.write.bind(this.terminal);
        this.terminal.write = (data, callback) => {
            // Add to scrollback buffer
            this.appendToBuffer(data);
            
            // Call original write
            return originalWrite(data, callback);
        };
        
        // Also capture when terminal buffer changes
        if (this.terminal.onLineFeed) {
            this.terminal.onLineFeed(() => {
                this.captureCurrentScreen();
            });
        }
    }
    
    appendToBuffer(data) {
        if (typeof data !== 'string') return;
        
        // Parse and store lines efficiently
        const lines = this.parseTerminalData(data);
        
        for (const line of lines) {
            this.scrollbackBuffer.push(line);
        }
        
        // Trim buffer if it exceeds max size
        if (this.scrollbackBuffer.length > this.maxBufferSize) {
            this.scrollbackBuffer = this.scrollbackBuffer.slice(-this.maxBufferSize);
        }
    }
    
    parseTerminalData(data) {
        // Split data into lines while preserving ANSI sequences
        const lines = [];
        let currentLine = '';
        let inEscapeSequence = false;
        let escapeBuffer = '';
        
        for (let i = 0; i < data.length; i++) {
            const char = data[i];
            
            if (char === '\x1b') {
                inEscapeSequence = true;
                escapeBuffer = char;
            } else if (inEscapeSequence) {
                escapeBuffer += char;
                // Check if escape sequence is complete
                if (/[a-zA-Z~]/.test(char)) {
                    currentLine += escapeBuffer;
                    inEscapeSequence = false;
                    escapeBuffer = '';
                }
            } else if (char === '\n' || char === '\r\n') {
                lines.push(currentLine);
                currentLine = '';
            } else if (char !== '\r') {
                currentLine += char;
            }
        }
        
        if (currentLine || escapeBuffer) {
            lines.push(currentLine + escapeBuffer);
        }
        
        return lines;
    }
    
    captureCurrentScreen() {
        // Capture the current visible screen content
        if (!this.terminal.buffer || !this.terminal.buffer.active) return;
        
        const buffer = this.terminal.buffer.active;
        const baseY = buffer.baseY || 0;
        const length = buffer.length || this.rows;
        
        for (let i = 0; i < length; i++) {
            const line = buffer.getLine(baseY + i);
            if (line) {
                const lineText = line.translateToString(true);
                if (lineText && lineText.trim()) {
                    // Only capture non-empty lines
                    this.scrollbackBuffer.push(lineText);
                }
            }
        }
        
        // Trim excess
        if (this.scrollbackBuffer.length > this.maxBufferSize) {
            this.scrollbackBuffer = this.scrollbackBuffer.slice(-this.maxBufferSize);
        }
    }
    
    enterScrollMode() {
        if (this.isScrolling) return;
        
        this.isScrolling = true;
        this.viewportOffset = 0;
        
        // Show overlay
        this.overlay.style.display = 'block';
        
        // Clear and populate scroll terminal with buffer
        this.scrollTerminal.clear();
        this.renderScrollback();
        
        // Add scroll mode indicator
        this.showScrollIndicator();
    }
    
    exitScrollMode() {
        if (!this.isScrolling) return;
        
        this.isScrolling = false;
        this.viewportOffset = 0;
        
        // Hide overlay
        this.overlay.style.display = 'none';
        
        // Hide scroll indicator
        this.hideScrollIndicator();
        
        // Return focus to main terminal
        this.terminal.focus();
    }
    
    scroll(direction, lines = 1) {
        if (!this.isScrolling) {
            this.enterScrollMode();
        }
        
        const totalLines = this.scrollbackBuffer.length;
        const visibleLines = this.rows;
        const maxOffset = Math.max(0, totalLines - visibleLines);
        
        if (direction === 'up') {
            this.viewportOffset = Math.min(this.viewportOffset + lines, maxOffset);
        } else {
            this.viewportOffset = Math.max(0, this.viewportOffset - lines);
        }
        
        this.renderScrollback();
        this.updateScrollIndicator();
        
        // Auto-exit scroll mode when at bottom
        if (this.viewportOffset === 0) {
            setTimeout(() => this.exitScrollMode(), 500);
        }
    }
    
    renderScrollback() {
        if (!this.scrollTerminal) return;
        
        this.scrollTerminal.clear();
        
        const totalLines = this.scrollbackBuffer.length;
        const startIdx = Math.max(0, totalLines - this.rows - this.viewportOffset);
        const endIdx = Math.min(totalLines, startIdx + this.rows);
        
        // Write visible portion of buffer to scroll terminal
        for (let i = startIdx; i < endIdx; i++) {
            const line = this.scrollbackBuffer[i];
            if (line !== undefined) {
                this.scrollTerminal.writeln(line);
            }
        }
    }
    
    showScrollIndicator() {
        if (this.scrollIndicator) return;
        
        this.scrollIndicator = document.createElement('div');
        this.scrollIndicator.id = 'scroll-indicator';
        this.scrollIndicator.style.cssText = `
            position: absolute;
            top: 10px;
            right: 20px;
            background: rgba(137, 180, 250, 0.9);
            color: #1e1e2e;
            padding: 6px 12px;
            border-radius: 6px;
            font-size: 12px;
            font-weight: bold;
            z-index: 100;
            box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
        `;
        
        const terminalContainer = document.getElementById('terminal');
        if (terminalContainer) {
            terminalContainer.appendChild(this.scrollIndicator);
        }
        
        this.updateScrollIndicator();
    }
    
    updateScrollIndicator() {
        if (!this.scrollIndicator) return;
        
        const totalLines = this.scrollbackBuffer.length;
        const currentLine = totalLines - this.viewportOffset;
        const percentage = Math.round((currentLine / totalLines) * 100);
        
        this.scrollIndicator.textContent = `SCROLL MODE (${percentage}%) - ESC to exit`;
    }
    
    hideScrollIndicator() {
        if (this.scrollIndicator) {
            this.scrollIndicator.remove();
            this.scrollIndicator = null;
        }
    }
    
    handleWheel(deltaY) {
        const lines = Math.ceil(Math.abs(deltaY) / 50); // Adjust sensitivity
        this.scroll(deltaY > 0 ? 'up' : 'down', lines);
    }
    
    handleKeyboard(key) {
        if (!this.isScrolling) return false;
        
        switch(key) {
            case 'Escape':
                this.exitScrollMode();
                return true;
            case 'PageUp':
                this.scroll('up', this.rows - 1);
                return true;
            case 'PageDown':
                this.scroll('down', this.rows - 1);
                return true;
            case 'ArrowUp':
                this.scroll('up', 1);
                return true;
            case 'ArrowDown':
                this.scroll('down', 1);
                return true;
            case 'Home':
                this.viewportOffset = this.scrollbackBuffer.length - this.rows;
                this.renderScrollback();
                this.updateScrollIndicator();
                return true;
            case 'End':
                this.viewportOffset = 0;
                this.renderScrollback();
                this.updateScrollIndicator();
                return true;
            case 'g':
                // Vim-style: gg to go to top
                if (this.lastKey === 'g') {
                    this.viewportOffset = this.scrollbackBuffer.length - this.rows;
                    this.renderScrollback();
                    this.updateScrollIndicator();
                }
                this.lastKey = 'g';
                setTimeout(() => { this.lastKey = null; }, 500);
                return true;
            case 'G':
                // Vim-style: G to go to bottom
                this.viewportOffset = 0;
                this.renderScrollback();
                this.updateScrollIndicator();
                return true;
            default:
                // Any other key exits scroll mode
                this.exitScrollMode();
                return false;
        }
    }
    
    resize(cols, rows) {
        this.cols = cols;
        this.rows = rows;
        
        if (this.scrollTerminal) {
            this.scrollTerminal.resize(cols, rows);
        }
        
        if (this.isScrolling) {
            this.renderScrollback();
        }
    }
    
    // Get the complete scrollback as text
    getFullScrollback() {
        return this.scrollbackBuffer.join('\n');
    }
    
    // Clear scrollback buffer
    clearScrollback() {
        this.scrollbackBuffer = [];
        this.viewportOffset = 0;
        if (this.isScrolling) {
            this.exitScrollMode();
        }
    }
}

// Export for use in terminal.js
window.ScrollbackManager = ScrollbackManager;