# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Monolith is a Rust library and CLI tool for saving web pages as single HTML files with all resources embedded. The project supports multiple execution modes and optional features through Cargo feature flags.

## Architecture

### Core Components

- **Core Processing**: `src/core.rs` - Main document processing logic with `create_monolithic_document()` and `create_monolithic_document_from_data()`
- **Environment Management**: `src/env.rs` - Type-safe environment variable system with validation
- **Network Layer**: `src/network/` - HTTP client, caching, session management, and cookie handling
- **Parsers**: `src/parsers/` - HTML (html5ever), CSS, JavaScript, and link rewriting
  - `src/parsers/html/` - Comprehensive HTML processing with DOM manipulation
  - `src/parsers/link_rewriter.rs` - URL rewriting and resource embedding
- **Translation System**: `src/translation/` - Optional translation pipeline with external API integration
  - `src/translation/core/` - Translation engine and service layer
  - `src/translation/pipeline/` - Text processing pipeline (collection, filtering, batching)
  - `src/translation/storage/` - Multi-level caching system
- **Web Server**: `src/web/` - Optional Axum-based web interface with REST API
  - `src/web/handlers/` - HTTP request handlers and API endpoints
  - `src/web/library/` - Document library management
- **Builders**: `src/builders/` - Output format builders (web feature only)

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
# Run all tests
make test
cargo test --locked

# Run tests for specific features
cargo test --features="translation" --locked
cargo test --features="web" --locked

# Run specific test modules
cargo test cli:: --locked
cargo test core:: --locked
cargo test translation:: --locked

# Run integration tests
cargo test --test integration --locked

# Run tests with test output
cargo test --locked -- --nocapture
```

### Code Quality
```bash
# Format code
make format
cargo fmt --all

# Lint code  
make lint
cargo clippy --fix --allow-dirty --allow-staged

# Check formatting and linting (CI-style checks)
make format-check
make lint-check

# Install and uninstall
make install
make uninstall

# Clean build artifacts
make clean

# Update dependencies
make update-lock-file
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

### Test Organization
- **Unit tests**: Inline `#[cfg(test)]` modules in source files
- **Integration tests**: `tests/` directory organized by functionality
- **Test data**: `tests/_data_/` contains realistic HTML, CSS, and JS test files
- **Feature-specific tests**: Use appropriate feature flags for testing optional functionality

### Test Data Categories
- `tests/_data_/basic/` - Basic HTML and resource files
- `tests/_data_/css/` - CSS parsing and embedding tests
- `tests/_data_/unusual_encodings/` - Character encoding tests (GB2312, ISO-8859-1)
- `tests/_data_/svg/` - SVG and vector graphics tests
- `tests/_data_/integrity/` - Resource integrity verification tests

### Test Modules
- `tests/cli/` - Command-line interface functionality
- `tests/core/` - Core processing logic tests
- `tests/html/` - HTML parsing and manipulation tests
- `tests/translation_pipeline.rs` - End-to-end translation functionality
- `tests/integration/translation/` - Translation system integration tests

## Error Handling

The codebase uses custom error types:
- `MonolithError` for core processing errors
- Translation errors in `src/translation/error.rs`
- Consistent error handling patterns throughout

## Code Style

- Uses rustfmt with configuration in `rustfmt.toml` (max_width=100, tab_spaces=4)
- Extensive documentation comments for public APIs
- Conditional compilation with `#[cfg(feature = "...")]`
- Follows standard Rust naming conventions
- Chinese comments preferred (使用中文编写注释)

## Project Structure

### Binary Targets
- `monolith` (CLI): Requires `cli` feature, entry point at `src/main.rs`
- `monolith-web` (Web server): Requires `web` feature, entry point at `src/web_main.rs`

### Environment Configuration
- `cargo/config.toml` - Cargo build configuration (OPENSSL_NO_VENDOR=1)
- `rustfmt.toml` - Code formatting rules
- `Makefile` - Development automation scripts

### Important Files
- `src/CLAUDE.md` - Rust coding standards and project guidelines
- `src/translation_legacy.rs` - Legacy translation support for backward compatibility

## Development Environment

### Prerequisites
- Rust 1.70+ (edition 2021)
- For web features: MongoDB (optional, for persistent storage)
- For translation features: External translation API access

### Environment Variables
The project supports extensive environment configuration through `src/env.rs`:
- Translation API keys and endpoints
- Web server configuration (host, port, theme)
- Cache settings and timeouts
- Debug and logging options

### OpenSSL Configuration
- Uses system OpenSSL by default (`OPENSSL_NO_VENDOR=1`)
- Use `vendored-openssl` feature for static linking when needed