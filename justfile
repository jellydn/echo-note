# EchoNote — Tauri v2 + React + Rust

set dotenv-load

# List available recipes
default:
    @just --list

# Start Tauri dev with hot reload
dev:
    cargo tauri dev

# Build release
build:
    cargo tauri build

# Run all checks (typecheck + lint + format)
check: check-ts check-rs

# TypeScript typecheck
check-ts:
    bun run typecheck

# Rust typecheck
check-rs:
    cargo check --manifest-path src-tauri/Cargo.toml

# Lint everything
lint: lint-ts lint-rs

# Lint TypeScript/JSON with Biome
lint-ts:
    npx @biomejs/biome check .

# Lint Rust with clippy
lint-rs:
    cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings

# Format everything
fmt: fmt-ts fmt-rs

# Format TypeScript/JSON with Biome
fmt-ts:
    npx @biomejs/biome check --write .

# Format Rust
fmt-rs:
    cargo fmt --manifest-path src-tauri/Cargo.toml

# Run pre-commit hooks on all files
pre-commit:
    prek run --all-files

# Run Rust tests
test-rs *args:
    cargo test --manifest-path src-tauri/Cargo.toml {{args}}

# Install dependencies
setup:
    bun install
    prek install
