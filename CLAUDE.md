# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is **r_ecipe_s** (Recipes), a full-stack recipe management application built with Rust. The project features a recipe storage system with search capabilities using PostgreSQL with ParadeDB full-text search and TensorChord vector search plugins.

## Architecture

The codebase is organized as a Rust workspace with the following key components:

### Backend Components
- **`r_ecipe_s_backend/`** - Core business logic library containing:
  - Database operations and migrations (PostgreSQL with sqlx)
  - Search functionality using ParadeDB and TensorChord plugins
  - Recipe service with CRUD operations
  - Authentication with JWT tokens
  - REST API endpoints using Axum
- **`server/`** - Main server executable that combines all backend services
- **`r_ecipe_s_model/`** - Shared data models and types used across frontend/backend

### Frontend Components  
- **`r_ecipe_s_frontend/`** - Core frontend library using Leptos framework
- **`frontend_ls/`** - Main frontend executable that renders to WebAssembly
- **Frontend uses Leptos for reactive UI with client-side rendering (CSR)**

### Key Technologies
- **Backend**: Rust, Axum, SQLx, PostgreSQL with ParadeDB (full-text search) and TensorChord (vector search), JWT auth
- **Frontend**: Rust, Leptos (WebAssembly), TailwindCSS  
- **Build Tools**: Cargo (Rust), Trunk (WebAssembly bundler), TailwindCSS
- **Deployment**: Docker, Fly.io

## Development Commands

### Prerequisites
Install required tools:
```bash
# Install sqlx CLI for database migrations
cargo install sqlx-cli --no-default-features --features postgres

# Install trunk for frontend builds (if not already installed)
cargo install trunk
```

### Database Setup
```bash
# Copy and configure the database config
cp server/config/config.toml.dist server/config/config.toml
# Edit config.toml with your database credentials

# Run database migrations (required before compilation)
cd r_ecipe_s_backend
sqlx database create  # if database doesn't exist
sqlx migrate run
```

### Development Server
```bash
# Run backend server (from project root)
cd server
cargo run

# Run frontend development server (separate terminal)
cd frontend_ls
trunk serve
# Frontend will be available at the address specified in Trunk.toml
```

### Building for Production
```bash
# Full production build (creates Docker image)
./build.sh

# Backend only
cd server
cargo build --release --target=x86_64-unknown-linux-gnu

# Frontend only
cd frontend_ls
trunk build --release
```

### Testing
```bash
# Run all tests in workspace
cargo test

# Run tests for specific crate
cd r_ecipe_s_backend
cargo test
```

### CSS Development
The project uses TailwindCSS which is automatically compiled via Trunk hooks during frontend builds.

## Configuration

### Environment Variables
- `API_KEY` - Bearer token for API authentication
- `R_ECIPE_S_DB_PASSWORD` - Database password (production)
- `RUST_LOG` - Logging configuration

### Config Files
- `server/config/config.toml` - Main application configuration (database, HTTP settings)
- `frontend_ls/Trunk.toml` - Frontend build configuration
- `frontend_ls/tailwind.config.js` - TailwindCSS configuration

## Database Schema

The application uses PostgreSQL with ParadeDB and TensorChord extensions:
- Recipe storage with vector embeddings for semantic search
- Full-text search capabilities via ParadeDB plugin
- Vector search functionality via TensorChord plugin

## API Architecture

The server exposes REST endpoints under `/api/v1/` and serves the frontend static files. The search system supports both traditional full-text search and semantic search via vector embeddings, all handled within PostgreSQL using the specialized plugins.

## Development Notes

- The backend requires running database migrations before compilation due to SQLx compile-time query checking
- Frontend builds automatically compile TailwindCSS via Trunk build hooks
- The server changes directory to `frontend_ls` at runtime to serve static assets
- Search functionality is now consolidated within PostgreSQL using ParadeDB and TensorChord plugins
- Vector embeddings are generated using a local ML model (MiniLM)