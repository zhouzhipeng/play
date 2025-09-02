# tmux Support for Web Terminal

This web terminal now includes full tmux session support, allowing you to create persistent terminal sessions that survive disconnections and can be accessed from multiple browser tabs.

## Features

### Session Management
- **Persistent Sessions**: Terminal sessions persist even when you close your browser
- **Multiple Sessions**: Create and manage multiple named sessions
- **Session Switching**: Seamlessly switch between different sessions
- **Auto-reconnect**: Automatically reconnect to your last session when you return

### tmux Integration
- **Automatic Detection**: The terminal automatically detects if tmux is installed
- **Graceful Fallback**: Works as a regular terminal if tmux is not available
- **Full tmux Features**: Access all tmux functionality including windows, panes, and commands

## Installation

### Prerequisites

To use tmux sessions, you need to have tmux installed on your server:

**macOS:**
```bash
brew install tmux
```

**Ubuntu/Debian:**
```bash
sudo apt-get install tmux
```

**CentOS/RHEL/Fedora:**
```bash
sudo yum install tmux
# or
sudo dnf install tmux
```

## Usage

### Web Interface

When tmux is available, you'll see a session management panel in the top-right corner of the terminal with:
- **Session List**: View all active sessions with their details
- **New Session**: Create a new named or auto-generated session
- **Connect**: Switch to a different session
- **Delete**: Remove sessions you no longer need

### WebSocket API

The terminal supports the following session-related messages:

#### Create Session
```javascript
ws.send(JSON.stringify({
    type: 'CreateSession',
    name: 'my-session' // optional
}));
```

#### List Sessions
```javascript
ws.send(JSON.stringify({
    type: 'ListSessions'
}));
```

#### Connect to Session
```javascript
ws.send(JSON.stringify({
    type: 'ConnectToSession',
    session_name: 'my-session'
}));
```

#### Delete Session
```javascript
ws.send(JSON.stringify({
    type: 'DeleteSession',
    name: 'my-session'
}));
```

### REST API

The following REST endpoints are available for session management:

#### List All Sessions
```bash
GET /web-terminal/api/sessions
```

#### Create New Session
```bash
POST /web-terminal/api/sessions
Content-Type: application/json

{
    "name": "my-session"  // optional
}
```

#### Delete Session
```bash
DELETE /web-terminal/api/sessions/{session_name}
```

## Architecture

### Backend Components

1. **SessionManager** (`session_manager.rs`)
   - Manages tmux session lifecycle
   - Tracks session metadata
   - Handles session creation, deletion, and attachment

2. **LocalTerminal** (`local_terminal.rs`)
   - Extended to support tmux session attachment
   - Falls back to regular shell if tmux unavailable
   - Handles both direct PTY and tmux-wrapped PTY

3. **WebSocket Handler** (`websocket.rs`)
   - New message types for session operations
   - Session state tracking per connection
   - Automatic session detachment on disconnect

4. **HTTP Server** (`server.rs`)
   - REST API endpoints for session management
   - Shared SessionManager across all connections
   - State management with Arc<SessionManager>

### Frontend Components

- **Session UI**: Interactive panel for session management
- **Auto-detection**: Automatically shows/hides based on tmux availability
- **Real-time Updates**: Session list updates automatically
- **Status Display**: Shows current session in connection status

## Benefits

1. **Persistence**: Never lose your work when connection drops
2. **Collaboration**: Multiple users can attach to the same session
3. **Organization**: Keep different projects in separate sessions
4. **Efficiency**: Switch between tasks without losing context
5. **Recovery**: Reconnect to sessions after server restarts

## Limitations

- tmux must be installed on the server for session features
- Session names must be unique
- Sessions persist until manually deleted or server restart
- Maximum number of sessions depends on system resources

## Troubleshooting

### tmux Not Detected
- Verify tmux is installed: `which tmux`
- Check tmux version: `tmux -V` (version 1.8+ recommended)
- Ensure tmux is in PATH

### Session Connection Issues
- Check if session exists: `tmux list-sessions`
- Verify no other client is exclusively attached
- Check system resources (memory, processes)

### Performance
- Limit number of concurrent sessions
- Close unused sessions to free resources
- Monitor system load with `top` or `htop`