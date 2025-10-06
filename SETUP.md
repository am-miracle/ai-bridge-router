# Bridge Router - Development Setup

## Prerequisites

- **Node.js** 18+ and npm
- **Rust** 1.70+ and Cargo
- **Git**

## Quick Start

### 1. Clone and Install

```bash
# Clone the repository
git clone <repository-url>
cd bridge-router

# Install root dependencies (Husky, Prettier, lint-staged)
npm install

# Install frontend dependencies
cd frontend
npm install
cd ..
```

### 2. Initialize Git Hooks

```bash
# Initialize Husky (from root directory)
npm run prepare

# Make hooks executable (Mac/Linux)
chmod +x .husky/pre-commit
chmod +x .husky/pre-push
chmod +x .husky/commit-msg
```

### 3. Environment Setup

Create `.env` files:

**Backend** (`/bridge-router/.env`):
```env
DATABASE_URL=your_database_url
REDIS_URL=redis://localhost:6379
RUST_LOG=info
```

**Frontend** (`/bridge-router/frontend/.env`):
```env
PUBLIC_API_URL=http://localhost:8080
```

### 4. Run Development Servers

```bash
# Terminal 1 - Backend
npm run dev:backend
# or
cargo run

# Terminal 2 - Frontend
npm run dev:frontend
# or
cd frontend && npm run dev
```

## Available Scripts

### Root Level Commands

```bash
# Development
npm run dev:frontend         # Start frontend dev server
npm run dev:backend          # Start backend dev server

# Formatting
npm run format               # Format all code (frontend + backend)
npm run format:frontend      # Format frontend only
npm run format:backend       # Format backend only (cargo fmt)
npm run format:check         # Check formatting (CI)
npm run format:check:frontend
npm run format:check:backend

# Linting
npm run lint:frontend        # Lint frontend code
npm run lint:backend         # Lint backend code (clippy)

# Type Checking
npm run type-check           # Type check frontend

# Testing
npm run test:backend         # Run Rust tests

# Building
npm run build:frontend       # Build frontend for production
npm run build:backend        # Build backend release binary
```

## Git Hooks

### Pre-commit Hook
Runs automatically before each commit:
- ✓ Formats staged files (Prettier for frontend, cargo fmt for backend)
- ✓ Lints staged files
- ✓ Type checks frontend

### Pre-push Hook
Runs automatically before pushing:
- ✓ Format check (frontend + backend)
- ✓ Lint check (frontend + backend)
- ✓ Type check (frontend)
- ✓ Build check (frontend)
- ✓ Tests (backend)

### Commit Message Hook
Enforces conventional commit format:
- ✓ `feat: add new feature`
- ✓ `fix: resolve bug`
- ✓ `docs: update documentation`
- ✓ Types: feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert

## Bypassing Hooks (Emergency Only)

```bash
# Skip pre-commit hooks
git commit --no-verify -m "fix: emergency fix"

# Skip pre-push hooks
git push --no-verify
```

**⚠️ Warning**: Only bypass hooks in emergencies. CI will still catch issues.

## GitHub Actions CI/CD

### Workflows

#### Full Stack CI (`fullstack-ci.yml`)
Runs on every push/PR:

**Frontend Jobs:**
- Lint & Format Check
- Type Check
- Build (with artifact upload)

**Backend Jobs:**
- Lint & Format Check (cargo fmt, clippy)
- Tests (with Redis service)
- Build Release Binary
- Security Audit (cargo-audit)

**Integration:**
- Integration tests (frontend + backend)
- Optional deployment job

#### Original Backend CI (`ci.yml`)
Comprehensive backend testing:
- Multi-platform builds (Linux, macOS, Windows)
- Security audits
- Coverage reports

### GitHub Secrets

Add these in your repository settings:

```
PUBLIC_API_URL          # Production API URL
TEST_DATABASE_URL       # Test database connection
TEST_REDIS_URL          # Test Redis connection (optional)
```

## Project Structure

```
bridge-router/
├── .github/
│   └── workflows/
│       ├── ci.yml              # Backend CI (original)
│       └── fullstack-ci.yml    # Full stack CI/CD
├── .husky/
│   ├── pre-commit              # Pre-commit hook
│   ├── pre-push                # Pre-push hook
│   └── commit-msg              # Commit message validation
├── frontend/
│   ├── src/                    # Frontend source code
│   ├── package.json            # Frontend dependencies
│   └── .env                    # Frontend env variables
├── src/                        # Backend source code
├── .prettierrc                 # Prettier config (root)
├── .prettierignore             # Prettier ignore patterns
├── package.json                # Root package.json
├── Cargo.toml                  # Rust dependencies
└── .env                        # Backend env variables
```

## Troubleshooting

### Husky hooks not running

```bash
# Reinstall Husky
npm run prepare
chmod +x .husky/*
```

### Prettier not formatting

```bash
# Check config
cat .prettierrc

# Format manually
npm run format
```

### Cargo clippy warnings

```bash
# Fix automatically
cargo clippy --fix

# Or allow warnings temporarily
cargo clippy --all-targets --all-features
```

### Frontend type errors

```bash
cd frontend
npm run type-check
```

### CI failing but local passes

- Check Node version matches CI (20.x)
- Check Rust version matches CI (stable)
- Run `npm run format:check` and `npm run lint:frontend`
- Run `cargo fmt --check` and `cargo clippy`

## Best Practices

1. **Write Meaningful Commits**: Use conventional commit format
2. **Keep Commits Small**: Focus on one change per commit
3. **Test Locally**: Run full checks before pushing
4. **Don't Bypass Hooks**: Only in genuine emergencies
5. **Review CI Logs**: Check GitHub Actions for warnings
6. **Update Dependencies**: Regularly update npm and cargo packages
7. **Document Changes**: Update README/docs when adding features

## Development Workflow

```bash
# 1. Create feature branch
git checkout -b feat/new-feature

# 2. Make changes
# ... edit files ...

# 3. Stage changes
git add .

# 4. Commit (triggers pre-commit hook)
git commit -m "feat: add new feature"
# Hook runs: format, lint, type-check

# 5. Push (triggers pre-push hook)
git push origin feat/new-feature
# Hook runs: format-check, lint, type-check, build, test

# 6. Create pull request
# GitHub Actions CI runs automatically

# 7. Merge after CI passes
```

## IDE Setup

### VS Code Extensions

```json
{
  "recommendations": [
    "esbenp.prettier-vscode",
    "dbaeumer.vscode-eslint",
    "astro-build.astro-vscode",
    "rust-lang.rust-analyzer",
    "tamasfe.even-better-toml"
  ]
}
```

### VS Code Settings

```json
{
  "editor.formatOnSave": true,
  "editor.defaultFormatter": "esbenp.prettier-vscode",
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "rust-analyzer.checkOnSave.command": "clippy"
}
```

## Getting Help

- Check GitHub Actions logs for CI failures
- Review pre-commit/pre-push output
- Run individual checks manually to isolate issues
- Create an issue for persistent problems
