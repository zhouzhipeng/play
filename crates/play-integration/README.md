# Play Integration Services

This directory contains integration modules for external services.

## Structure

- **play-integration-xiaozhi** - Xiaozhi AI MCP (Model Context Protocol) client integration

## Purpose

These crates provide integrations with external AI services and APIs:
- WebSocket-based communication with AI services
- MCP protocol implementation for AI model interactions
- Session management and message routing

## Future Integrations

This directory is designed to accommodate additional integrations:
- Other AI service providers
- External API clients
- Third-party service connectors

## Usage

Integration modules are loaded by play-server when their corresponding features are enabled in Cargo.toml.

## Development

To add a new integration:
1. Create a new crate under this directory (e.g., `play-integration-newservice`)
2. Implement the required protocol and client logic
3. Add it as an optional dependency to play-server
4. Enable with a feature flag