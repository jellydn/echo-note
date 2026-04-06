# Tech Stack

## Languages & Runtimes
- Rust (2021 edition) - backend/Tauri core
- TypeScript 5.8.3 - frontend
- JavaScript/ES2020 - frontend runtime

## Frameworks & Libraries (Frontend)
- React 19.1.0
- @tauri-apps/api 2.x - Tauri integration
- @tauri-apps/plugin-opener 2.x - URL opening

## Frameworks & Libraries (Backend/Rust)
- tauri 2 - desktop framework
- tauri-build 2 - build tooling
- tauri-plugin-opener 2 - external link handler
- sqlx 0.8 - async SQL toolkit with SQLite support
- tokio 1.x (full features) - async runtime
- serde 1 - serialization/deserialization
- serde_json 1 - JSON handling
- chrono 0.4 - datetime handling
- anyhow 1 - error handling
- reqwest 0.12 - HTTP client with streaming
- futures-util 0.3 - async utilities
- whisper-rs 0.13 - speech-to-text via Whisper
- cpal 0.15 - audio capture/playback
- hound 3.5 - WAV file reading/writing
- log 0.4 - logging framework

## Build & Tooling
- Vite 7.0.4 - frontend bundler
- Tauri CLI 2 - desktop app build and dev
- TypeScript compiler (tsc) - type checking
- Biome 2.4.10 - linting and formatting
- Bun - package manager (configured in tauri.conf.json)
- Cargo - Rust package manager

## Configuration Files
- `src-tauri/tauri.conf.json` - Tauri app configuration (window, bundle, security)
- `src-tauri/Cargo.toml` - Rust dependencies and build settings
- `package.json` - Node.js dependencies and scripts
- `vite.config.ts` - Frontend build configuration
- `tsconfig.json` - TypeScript configuration
- `tsconfig.node.json` - TypeScript config for Node.js context
- `biome.json` - Biome linting/formatting rules

## Package Management
- Bun (primary, specified in tauri.conf.json)
- Cargo (Rust)
