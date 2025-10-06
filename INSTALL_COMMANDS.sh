#!/bin/bash

# Bridge Router - One-Command Installation Script
# This script sets up the entire development environment

set -e  # Exit on error

echo "🚀 Bridge Router - Development Environment Setup"
echo "================================================"
echo ""

# Check prerequisites
echo "📋 Checking prerequisites..."

# Check Node.js
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed. Please install Node.js 18+ first."
    exit 1
fi
echo "✓ Node.js $(node --version)"

# Check npm
if ! command -v npm &> /dev/null; then
    echo "❌ npm is not installed."
    exit 1
fi
echo "✓ npm $(npm --version)"

# Check Cargo
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo is not installed. Please install Rust first."
    exit 1
fi
echo "✓ Cargo $(cargo --version)"

# Check Git
if ! command -v git &> /dev/null; then
    echo "❌ Git is not installed."
    exit 1
fi
echo "✓ Git $(git --version)"

echo ""
echo "📦 Installing dependencies..."

# Install root dependencies
echo "  → Installing root dependencies (Husky, Prettier, lint-staged)..."
npm install

# Install frontend dependencies
echo "  → Installing frontend dependencies..."
cd frontend
npm install
cd ..

echo ""
echo "🔧 Setting up Git hooks..."

# Initialize Husky
npm run prepare

# Make hooks executable (Unix/Mac)
if [[ "$OSTYPE" == "linux-gnu"* ]] || [[ "$OSTYPE" == "darwin"* ]]; then
    chmod +x .husky/pre-commit
    chmod +x .husky/pre-push
    chmod +x .husky/commit-msg
    echo "✓ Hooks made executable"
fi

echo ""
echo "🎨 Testing Prettier..."
npm run format:check:frontend > /dev/null 2>&1 && echo "✓ Prettier check passed" || echo "⚠ Prettier check failed (might need formatting)"

echo ""
echo "🦀 Testing Rust formatting..."
cargo fmt --check > /dev/null 2>&1 && echo "✓ Rust formatting check passed" || echo "⚠ Rust formatting check failed (might need formatting)"

echo ""
echo "✅ Installation complete!"
echo ""
echo "📝 Next steps:"
echo ""
echo "1. Set up environment variables:"
echo "   - Copy .env.example to .env (if exists)"
echo "   - Add your DATABASE_URL, REDIS_URL, etc."
echo ""
echo "2. Start development servers:"
echo "   Terminal 1: npm run dev:backend"
echo "   Terminal 2: npm run dev:frontend"
echo ""
echo "3. Make your first commit:"
echo "   git add ."
echo "   git commit -m \"chore: initial setup\""
echo ""
echo "🔗 Helpful commands:"
echo "   npm run format              - Format all code"
echo "   npm run lint:frontend       - Lint frontend"
echo "   npm run lint:backend        - Lint backend (clippy)"
echo "   npm run test:backend        - Run backend tests"
echo "   npm run build:frontend      - Build frontend"
echo "   npm run build:backend       - Build backend"
echo ""
echo "📚 Read SETUP.md for detailed documentation"
echo ""
echo "Happy coding! 🎉"
