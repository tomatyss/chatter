#!/bin/bash

# Build script for Chatter - Gemini AI Chat CLI

set -e

echo "🔨 Building Chatter..."

# Build in release mode
cargo build --release

echo "✅ Build completed successfully!"

# Check if we should install
if [[ "$1" == "--install" ]]; then
    echo "📦 Installing chatter to /usr/local/bin..."
    
    # Check if we have permission to write to /usr/local/bin
    if [[ -w "/usr/local/bin" ]]; then
        cp target/release/chatter /usr/local/bin/
        echo "✅ chatter installed to /usr/local/bin/chatter"
    else
        echo "🔐 Need sudo permission to install to /usr/local/bin"
        sudo cp target/release/chatter /usr/local/bin/
        echo "✅ chatter installed to /usr/local/bin/chatter"
    fi
    
    echo ""
    echo "🎉 Installation complete!"
    echo ""
    echo "To get started:"
    echo "1. Get your Gemini API key from: https://aistudio.google.com/app/apikey"
    echo "2. Set it up: chatter config set-api-key"
    echo "3. Start chatting: chatter"
    echo ""
elif [[ "$1" == "--help" ]] || [[ "$1" == "-h" ]]; then
    echo ""
    echo "Usage: ./build.sh [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --install    Build and install to /usr/local/bin"
    echo "  --help, -h   Show this help message"
    echo ""
    echo "Examples:"
    echo "  ./build.sh              # Just build"
    echo "  ./build.sh --install    # Build and install"
    echo ""
else
    echo ""
    echo "Binary built at: target/release/chatter"
    echo ""
    echo "To install system-wide, run:"
    echo "  ./build.sh --install"
    echo ""
    echo "Or copy manually:"
    echo "  sudo cp target/release/chatter /usr/local/bin/"
    echo ""
fi
