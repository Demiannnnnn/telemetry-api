<div align="center">

# 🛰️ Telemetry API

**Enterprise Device Telemetry Platform with End-to-End Encryption**

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-0.7-blue)](https://github.com/tokio-rs/axum)
[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-16-blue?logo=postgresql)](https://www.postgresql.org/)
[![License](https://img.shields.io/badge/License-Proprietary-red)]()

*A secure, privacy-first REST API for enterprise device telemetry collection, acting as an encrypted middleware between admin and worker applications.*

</div>

---

## 📋 Table of Contents

- [Overview](#overview)
- [Key Features](#key-features)
- [Architecture](#architecture)
- [Tech Stack](#tech-stack)
- [Project Structure](#project-structure)
- [Getting Started](#getting-started)
- [API Overview](#api-overview)
- [Security Model](#security-model)
- [Data Collection](#data-collection)
- [Legal Compliance](#legal-compliance)
- [Documentation](#documentation)
- [Contributing](#contributing)

---

## Overview

**Telemetry API** is a high-performance REST API built with **Rust** and **Axum** that serves as a secure middleware layer between two client applications:

- **Admin App** — Used by administrators to view telemetry metrics and manage workers
- **Worker App** — Installed on enterprise devices to collect and transmit telemetry data (with explicit worker consent)

The API follows a **zero-knowledge architecture**: all telemetry data is encrypted end-to-end (E2E) between the Admin and Worker apps. The server stores and relays encrypted payloads **without the ability to decrypt or inspect the data**. Encryption keys are managed exclusively on the client side.

### Why This Matters

In enterprise environments, monitoring device telemetry is essential for productivity analysis, security compliance, and operational efficiency. However, this must be balanced with **worker privacy rights** and **data protection regulations**. Our E2E encryption model ensures that:

1. The server operator **cannot access** raw telemetry data
2. Only the authorized admin with the correct decryption key can view metrics
3. Full compliance with **Chilean data protection laws** (Ley 19.628 & Ley 21.719)
4. Workers provide **explicit, informed consent** before any data collection begins

---

## Key Features

### 🔐 Security & Privacy
- **End-to-End Encryption** — Data encrypted on the worker device, decrypted only by the admin
- **Zero-Knowledge Server** — The API never has access to plaintext telemetry data
- **JWT Authentication** — Secure token-based authentication for all endpoints
- **Role-Based Access Control** — Strict separation between admin and worker permissions

### 📊 Telemetry Collection
- **System Activity** — Idle time, active applications, window titles, CPU/RAM/network usage, USB events
- **Network Activity** — Visited domains, domain categorization (work, social media, entertainment)
- **Productivity Metrics** — Keystroke frequency (not content), mouse click frequency, periodic screenshots
- **File Activity** — Access and modification logs for corporate directories
- **Location Tracking** — GPS data during work hours (enterprise devices only, with explicit consent)

### 🏗️ Architecture
- **Modular Design** — Clean separation by domain modules, not services
- **Middleware Pattern** — Acts as an encrypted relay between admin and worker apps
- **Entity-Based Backend** — Each domain entity has its own module with models, handlers, and routes
- **Database-First** — PostgreSQL with SQLx for compile-time query verification

### ⚡ Performance
- **Async Runtime** — Built on Tokio for high-concurrency workloads
- **Connection Pooling** — Efficient database connection management
- **Rate Limiting** — Configurable per-endpoint rate limiting
- **Batch Ingestion** — Support for bulk telemetry data submission

---

## Architecture

```
┌─────────────────┐         ┌─────────────────────┐         ┌──────────────────┐
│                 │         │                     │         │                  │
│   Worker App    │────────▶│   Telemetry API     │◀────────│   Admin App      │
│  (Data Source)  │  E2E    │   (Middleware)       │  E2E    │  (Data Viewer)   │
│                 │ Encrypt │                     │ Encrypt │                  │
└─────────────────┘         │  ┌───────────────┐  │         └──────────────────┘
                            │  │  PostgreSQL   │  │
                            │  │  (Encrypted   │  │
                            │  │   Storage)    │  │
                            │  └───────────────┘  │
                            └─────────────────────┘

         ┌──────────────────────────────────────────────┐
         │  🔑 E2E Keys managed CLIENT-SIDE only        │
         │  🔒 Server stores ONLY encrypted payloads    │
         │  👁️ Server has ZERO visibility into data     │
         └──────────────────────────────────────────────┘
```

### Entity Relationship

```
Admin (1) ◄────────► (N) Worker
                          │
                          ├── SystemActivity (idle, apps, windows, resources, usb)
                          ├── NetworkActivity (domains, categories)
                          ├── ProductivityMetrics (keystrokes, clicks, screenshots)
                          ├── FileActivity (access logs, modification logs)
                          └── LocationData (GPS coordinates, work hours only)
```

---

## Tech Stack

| Component       | Technology                          |
|----------------|-------------------------------------|
| Language        | Rust 1.75+                         |
| Web Framework   | Axum 0.7                           |
| Async Runtime   | Tokio                              |
| Database        | PostgreSQL 16                      |
| DB Driver       | SQLx (with compile-time checks)    |
| Authentication  | JWT (jsonwebtoken)                 |
| Serialization   | serde / serde_json                 |
| Validation      | validator                          |
| Logging         | tracing / tracing-subscriber       |
| Error Handling  | thiserror / anyhow                 |
| Configuration   | dotenvy / config                   |

---

## Project Structure

```
telemetry-api/
├── .ai-agents/                  # AI agent configuration and policies
│   ├── architecture.md          # Architecture decisions and patterns
│   ├── constraints.md           # Project constraints and boundaries
│   ├── conventions.md           # Code conventions and best practices
│   └── domain.md                # Domain model and business rules
│
├── docs/                        # Comprehensive documentation
│   ├── architecture-overview.md # System architecture and diagrams
│   ├── database-schema.md       # Complete database schema design
│   ├── api-specification.md     # REST API endpoint specification
│   ├── security-model.md        # E2E encryption and security design
│   ├── legal-compliance.md      # Chilean data protection compliance
│   └── data-dictionary.md       # Data dictionary for all entities
│
├── migrations/                  # SQLx database migrations
│
├── src/
│   ├── main.rs                  # Application entry point
│   ├── lib.rs                   # Library root, module declarations
│   ├── config/                  # Application configuration
│   │   └── mod.rs
│   ├── database/                # Database connection and pooling
│   │   └── mod.rs
│   ├── errors/                  # Custom error types and handlers
│   │   └── mod.rs
│   ├── middleware/               # Custom middleware (auth, logging, rate-limit)
│   │   └── mod.rs
│   ├── models/                  # Shared data models and DTOs
│   │   └── mod.rs
│   ├── routes/                  # Route definitions and grouping
│   │   └── mod.rs
│   │
│   ├── auth/                    # Authentication & authorization module
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── handlers.rs
│   │   └── routes.rs
│   │
│   ├── admin/                   # Admin entity module
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── handlers.rs
│   │   └── routes.rs
│   │
│   ├── worker/                  # Worker entity module
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── handlers.rs
│   │   └── routes.rs
│   │
│   ├── telemetry/               # Telemetry data module (core)
│   │   ├── mod.rs
│   │   ├── models.rs
│   │   ├── handlers.rs
│   │   ├── routes.rs
│   │   ├── system_activity.rs   # System/device activity models
│   │   ├── network_activity.rs  # Network/browsing activity models
│   │   ├── productivity.rs      # Productivity metrics models
│   │   ├── file_activity.rs     # File access/modification models
│   │   └── location.rs          # GPS/location models
│   │
│   └── crypto/                  # Encryption utilities and helpers
│       └── mod.rs
│
├── tests/                       # Integration tests
│
├── .env.example                 # Environment variable template
├── .gitignore
├── AGENT.md                     # AI agent instructions
├── Cargo.toml                   # Rust project manifest
└── README.md                    # This file
```

---

## Getting Started

### Prerequisites

- **Rust** 1.75 or higher ([install](https://rustup.rs/))
- **PostgreSQL** 16 or higher
- **SQLx CLI** (`cargo install sqlx-cli`)

### Setup

```bash
# 1. Clone the repository
git clone <repository-url>
cd telemetry-api

# 2. Copy environment configuration
cp .env.example .env
# Edit .env with your database credentials and configuration

# 3. Create the database
createdb telemetry_db

# 4. Run migrations
sqlx migrate run

# 5. Build and run
cargo run
```

The server will start on `http://localhost:8080` by default.

### Development

```bash
# Run with hot-reload (requires cargo-watch)
cargo watch -x run

# Run tests
cargo test

# Check SQL queries at compile time
cargo sqlx prepare

# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings
```

---

## API Overview

The API is organized around the following resource groups:

| Group              | Base Path              | Description                                |
|--------------------|------------------------|--------------------------------------------|
| Authentication     | `/api/v1/auth`         | Login, registration, token refresh         |
| Admin              | `/api/v1/admins`       | Admin profile management                   |
| Workers            | `/api/v1/workers`      | Worker registration and management         |
| System Activity    | `/api/v1/telemetry/system`     | Idle time, apps, windows, resources, USB |
| Network Activity   | `/api/v1/telemetry/network`    | Domains visited, categories              |
| Productivity       | `/api/v1/telemetry/productivity` | Keystroke/click frequency, screenshots |
| File Activity      | `/api/v1/telemetry/files`      | File access and modification logs        |
| Location           | `/api/v1/telemetry/location`   | GPS data during work hours               |
| Health             | `/api/v1/health`       | Service health check                       |

> 📖 Full API specification available in [`docs/api-specification.md`](docs/api-specification.md)

---

## Security Model

### End-to-End Encryption Flow

```
Worker Device                    Server                      Admin Device
─────────────                    ──────                      ────────────
     │                              │                             │
     │  1. Generate telemetry data  │                             │
     │                              │                             │
     │  2. Encrypt with shared key  │                             │
     │         (client-side)        │                             │
     │                              │                             │
     │  3. POST /telemetry/*  ─────▶│  4. Store encrypted blob   │
     │     (encrypted payload)      │     in PostgreSQL           │
     │                              │                             │
     │                              │◀──── 5. GET /telemetry/*    │
     │                              │      (request data)         │
     │                              │                             │
     │                              │─────▶ 6. Return encrypted   │
     │                              │       payload               │
     │                              │                             │
     │                              │       7. Decrypt with       │
     │                              │       shared key            │
     │                              │       (client-side)         │
```

### Key Security Principles

1. **Zero-Knowledge Architecture** — The server never possesses decryption keys
2. **Encrypted at Rest** — All telemetry data stored as encrypted blobs in PostgreSQL
3. **Encrypted in Transit** — TLS 1.3 for all API communications
4. **Authentication** — JWT-based with short-lived access tokens and refresh tokens
5. **Authorization** — Role-based access control (Admin vs Worker)
6. **Rate Limiting** — Per-endpoint rate limiting to prevent abuse
7. **Input Validation** — Strict validation on all request payloads
8. **Audit Logging** — All data access operations are logged (without exposing data content)

> 🔐 Detailed security documentation in [`docs/security-model.md`](docs/security-model.md)

---

## Data Collection

All telemetry data collection requires **explicit, informed worker consent**. The following categories are collected:

| Category            | Data Points                                           | Sensitivity | Consent Level     |
|---------------------|-------------------------------------------------------|-------------|-------------------|
| System Activity     | Idle time, active apps, window titles, CPU/RAM/Network, USB events | Medium      | Standard consent  |
| Network Activity    | Visited domains (not page content), domain categories | Medium      | Standard consent  |
| Productivity        | Keystroke frequency (NOT content), click frequency    | Medium      | Standard consent  |
| Screenshots         | Periodic screen captures (blurred/low-frequency)      | High        | Explicit consent  |
| File Activity       | Access/modification logs (corporate dirs only)        | Medium      | Standard consent  |
| Location (GPS)      | Coordinates during work hours (field roles only)      | Very High   | Explicit consent + operational justification |

> ⚠️ **Important distinctions:**
> - Keystroke **frequency** is collected, **never** keystroke content (this is NOT a keylogger)
> - Domain **URLs** are collected, **never** page content
> - File **access logs** are collected, **never** file contents
> - Screenshots require **separate explicit consent** and visible capture notice
> - GPS is **only** during work hours and **only** for field roles with clear operational need

---

## Legal Compliance

This platform is designed to comply with Chilean data protection legislation:

- **Ley 19.628** — Protección de la Vida Privada (Protection of Private Life)
- **Ley 21.719** — Ley de Protección de Datos Personales (Personal Data Protection Law, modernizing 19.628)

### Compliance Highlights

- ✅ Explicit, informed consent before any data collection
- ✅ Purpose limitation — data collected only for stated telemetry purposes
- ✅ Data minimization — only necessary data points collected
- ✅ Right to access — workers can request their data
- ✅ Right to deletion — workers can request data deletion
- ✅ Data breach notification procedures
- ✅ End-to-end encryption as a privacy-by-design measure
- ✅ Privacy impact assessment documentation

> 📜 Full legal compliance documentation in [`docs/legal-compliance.md`](docs/legal-compliance.md)

---

## Documentation

| Document | Description |
|----------|-------------|
| [`docs/architecture-overview.md`](docs/architecture-overview.md) | System architecture, diagrams, and design decisions |
| [`docs/database-schema.md`](docs/database-schema.md) | Complete database schema with entity relationships |
| [`docs/api-specification.md`](docs/api-specification.md) | Full REST API endpoint specification |
| [`docs/security-model.md`](docs/security-model.md) | Security architecture and E2E encryption model |
| [`docs/legal-compliance.md`](docs/legal-compliance.md) | Legal framework and data protection compliance |
| [`docs/data-dictionary.md`](docs/data-dictionary.md) | Data dictionary for all entities and fields |
| [`AGENT.md`](AGENT.md) | AI agent instructions for development |

---

## Contributing

1. Create a feature branch from `develop`
2. Follow the conventions in [`.ai-agents/conventions.md`](.ai-agents/conventions.md)
3. Ensure all tests pass: `cargo test`
4. Run linting: `cargo clippy -- -D warnings`
5. Format code: `cargo fmt`
6. Submit a pull request to `develop`

### Branch Strategy

| Branch             | Purpose                              |
|--------------------|--------------------------------------|
| `main`             | Production-ready releases            |
| `develop`          | Integration branch                   |
| `dev/architecture` | Architecture and documentation work  |
| `dev/features`     | Feature development                  |

---

<div align="center">

**Built with 🦀 Rust** | **Secured with 🔐 E2E Encryption** | **Compliant with 🇨🇱 Chilean Law**

</div>
