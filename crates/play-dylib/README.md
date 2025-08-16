# Play Dylib Plugins

This directory contains the dynamic library plugin system for Play Server.

## Structure

- **play-dylib-abi** - Application Binary Interface definitions for plugins
- **play-dylib-loader** - Runtime plugin loader with hot-reloading support
- **play-dylib-example** - Example plugin implementation

## Purpose

These crates provide a plugin system that allows:
- Dynamic loading of external libraries at runtime
- Hot-reloading of plugins during development
- Stable ABI for plugin communication
- Isolation of plugin code from the main server

## Usage

Plugins are loaded by the play-server when the `play-dylib-loader` feature is enabled.

## Development

To create a new plugin:
1. Implement the traits defined in `play-dylib-abi`
2. Build as a dynamic library (cdylib)
3. Place in the configured plugin directory
4. The server will automatically load it at startup