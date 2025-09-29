# AI-Optimized Bridge Router

## Overview

The AI-Optimized Bridge Router is a decentralized application (DApp) that helps users move assets across multiple blockchains securely, quickly, and at the lowest possible cost.

Today, users face a fragmented experience when bridging assets:

* They must manually compare fees, speed, and reliability across multiple bridges.
* They often worry about the security of each bridge, especially after historical exploits.
* Switching between multiple UIs and RPCs adds unnecessary friction.

This project solves these issues by aggregating available cross-chain bridge services into a single interface and applying real-time analytics and AI-driven scoring to recommend the best route for a given transfer.


## Key Features

* **Bridge Aggregation**
  Connects to top bridging protocols (Connext, Hop, Axelar, Wormhole, etc.) and fetches real-time data such as fees, liquidity, and finality times.

* **Quote Comparison**
  Displays normalized bridge quotes side by side, allowing users to quickly compare costs, speeds, and risks.

* **Security Scoring**
  Uses a heuristic scoring system based on audits, exploit history, and custodial risk. Later versions may include AI-based scoring.

* **Route Recommendation**
  Combines cost, speed, and security metrics into a weighted ranking system. Users can adjust their preferences (e.g., prioritize cheapest vs. safest).

* **Transaction Simulation**
  Previews expected gas costs, target contract addresses, and warnings before a transfer is initiated.

* **Unified Claim & Execution**
  Provides a single button for routing or claiming, redirecting seamlessly to the user’s wallet or the selected bridge’s execution flow.

---

## Technical Architecture

### Backend (Rust)

The backend is implemented in Rust for performance, safety, and scalability. It is built on:

* **Axum** for the web framework and routing.
* **Tokio** for asynchronous runtime.
* **SQLx** for PostgreSQL database interaction.
* **Redis** for caching frequently requested bridge data.
* **Tracing** for structured logging and observability.

The backend handles:

* Fetching and normalizing data from multiple bridge APIs.
* Managing a historical database of exploits, audits, and bridge reliability.
* Exposing REST API endpoints for quotes, health checks, and recommendations.
* Running the scoring engine to evaluate and rank bridge routes.

### Data Sources

* **Bridge APIs**: Direct integration with major bridges (Connext, Hop, Axelar, Wormhole).
* **Security Metadata**: Aggregated from audit reports, public incident reports, and trusted registries.
* **Gas and Chain Metrics**: Pulled from RPC endpoints and block explorers.


## Future Extensions

* Machine learning-based scoring (trained on historical data).
* Multi-hop routing across chains.
* Expansion to 10+ bridges.
* Telegram/Discord notifications when a better route becomes available.
* Smart contract: router contract for atomic multi-hop transfers

## Development Guidelines

### Folder Structure (Backend)

```
src/
├── main.rs            # Axum app entry point
├── config/            # Environment & config handling
├── db/                # SQLx connection & migrations
├── cache/             # Redis integration
├── routes/            # API route handlers
├── services/          # Business logic (bridges, scoring, recommendation)
├── models/            # Database models and DTOs
├── utils/             # Helpers and error handling
└── telemetry/         # Logging and metrics
```

### Code Standards

* All APIs return JSON using `serde::Serialize`.
* Errors use `thiserror` and `anyhow` for context-rich handling.
* Logging uses `tracing` with structured events.
* Secrets are loaded via `dotenvy` and `config`, never hardcoded.
* Integration tests live in `/tests` with real API health checks.

## Contribution Guide

We welcome contributions! To keep the project organized, please follow these steps:

### Setup

1. Clone the repo:
2. Copy `.env.example` to `.env` and fill in required values.
3. Run database migrations:

   ```bash
   cargo sqlx migrate run
   ```
4. Start backend:

   ```bash
   cargo run
   ```
5. Start frontend (from `frontend/`):

   ```bash
   npm install
   npm run dev
   ```

### Branching Model

* `main`: Stable branch.
* `dev`: Active development branch.
* Feature branches: `feature/<name>` (e.g., `feature/bridge-aggregator`).
* Bugfix branches: `fix/<name>`.

### Commit Style

Follow [Conventional Commits](https://www.conventionalcommits.org/):

* `feat:` new feature
* `fix:` bug fix
* `chore:` maintenance tasks
* `docs:` documentation changes
* `refactor:` code changes without new features

Example:

```
feat(bridge-client): add Connext API integration
```

### Code Review

* Open a PR against `dev`.
* Ensure CI passes (tests + lint).
* Request at least one reviewer.

### Testing

* Write integration tests for new endpoints in `/tests`.
* Run tests before PR submission:

  ```bash
  cargo test
  ```

### Issues

* Check open issues before creating a new one.
* Use labels (`backend`, `frontend`, `security`, `docs`).