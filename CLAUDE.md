# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Monolith is a Rust library and CLI tool for saving web pages as single HTML files with all resources embedded. The project supports multiple execution modes and optional features through Cargo feature flags.

## Architecture

### Core Components

- **Core Processing**: `src/core.rs` - Main document processing logic with `create_monolithic_document()` and `create_monolithic_document_from_data()`
- **Network Layer**: `src/network/` - HTTP client, caching, session management, and cookie handling
- **Parsers**: `src/parsers/` - HTML (html5ever), CSS, JavaScript, and link rewriting
- **Translation System**: `src/translation/` - Optional translation pipeline with external API integration
- **Web Server**: `src/web/` - Optional Axum-based web interface with REST API

### Execution Modes

1. **CLI Tool** (`src/main.rs`): Command-line interface with extensive options
2. **Web Server** (`src/web_main.rs`): HTTP server with web interface

### Feature System

The project uses Cargo features for optional functionality:

- `cli`: Command-line tool (default)
- `web`: Web server with Axum, MongoDB support
- `translation`: Translation functionality with markdown-translator integration
- `vendored-openssl`: Static OpenSSL linking

## Development Commands

### Building
```bash
# Standard CLI build
make build
cargo build --locked

# GUI functionality has been removed to reduce dependencies

# Web server
cargo build --bin monolith-web --features="web"

# Translation enabled
cargo build --features="translation"
```

### Testing
```bash
make test
cargo test --locked
```

### Code Quality
```bash
# Format code
make format
cargo fmt --all

# Lint code  
make lint
cargo clippy --fix --allow-dirty --allow-staged

# Check formatting and linting
make format-check
make lint-check
```

### Running Applications
```bash
# CLI tool
cargo run --bin monolith --features="cli" -- <URL>

# Web server
cargo run --bin monolith-web --features="web"

# GUI application removed
```

## Key Dependencies

- **html5ever**: HTML parsing and DOM manipulation
- **reqwest**: HTTP client for network requests
- **axum**: Web framework (web feature)
# GUI framework druid removed
- **clap**: Command-line parsing (CLI feature)
- **markdown-translator**: Translation integration (translation feature)
- **mongodb**: Database support (web feature)

## Working with Features

Always use appropriate feature flags when building or testing specific functionality:

```bash
# For translation work
cargo build --features="translation"
cargo test --features="translation"

# For web development  
cargo build --features="web,translation"
cargo test --features="web"

# GUI functionality removed
```

## Translation System

When working with the translation functionality:

- Configuration files use TOML format with .env support
- Translation API integration in `src/translation/service.rs`
- DOM text extraction in `src/translation/collector.rs`
- Caching system in `src/translation/cache.rs`
- Batch processing in `src/translation/batch.rs`

## Web Interface

The web server provides:

- REST API endpoints in `src/web/handlers/api/`
- HTML templates in `templates/`
- Theme management system with CSS variables
- MongoDB integration for persistent storage
- Static asset serving

## Testing Structure

- Unit tests: Inline `#[cfg(test)]` modules
- Integration tests: `tests/` directory with realistic test data in `tests/_data_/`
- Test different features with appropriate flags

## Error Handling

The codebase uses custom error types:
- `MonolithError` for core processing errors
- Translation errors in `src/translation/error.rs`
- Consistent error handling patterns throughout

## Code Style

- Uses rustfmt with configuration in `rustfmt.toml`
- Extensive documentation comments for public APIs
- Conditional compilation with `#[cfg(feature = "...")]`
- Follows standard Rust naming conventions