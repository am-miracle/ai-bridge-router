#!/bin/bash
# Setup script for pre-commit hooks

set -e

echo " Setting up pre-commit hooks for Bridge Router..."

# Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo " pre-commit is not installed. Installing..."

    # Try different installation methods
    if command -v pip &> /dev/null; then
        pip install pre-commit
    elif command -v pip3 &> /dev/null; then
        pip3 install pre-commit
    elif command -v brew &> /dev/null; then
        brew install pre-commit
    else
        echo " Please install pre-commit manually:"
        echo "   pip install pre-commit"
        echo "   or"
        echo "   brew install pre-commit"
        exit 1
    fi
fi

echo " pre-commit is installed"

# Install the pre-commit hooks
echo " Installing pre-commit hooks..."
pre-commit install

# Run pre-commit on all files to test
echo " Testing pre-commit hooks..."
pre-commit run --all-files

echo " Pre-commit hooks setup complete!"
echo ""
echo "Now every commit will automatically:"
echo "  - Format your Rust code with rustfmt"
echo "  - Run clippy linter"
echo "  - Check for security vulnerabilities"
echo "  - Validate file formats and syntax"
echo ""
echo "To run hooks manually: pre-commit run --all-files"
