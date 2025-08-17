#!/bin/bash

# Script to build gst-plugins-rs with LiveKit feature enabled on macOS

set -e

echo "=== Building gst-plugins-rs with LiveKit support ==="
echo "Note: This installs to ~/.local/lib/gstreamer-1.0 to avoid Homebrew conflicts"
echo ""

BUILD_DIR="/tmp/gst-plugins-rs-build"
mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

# Clone the repository
if [ ! -d "gst-plugins-rs" ]; then
    echo "Cloning gst-plugins-rs repository..."
    git clone https://gitlab.freedesktop.org/gstreamer/gst-plugins-rs.git
    cd gst-plugins-rs
else
    cd gst-plugins-rs
    echo "Updating existing repository..."
    git fetch
    git pull
fi

echo "Building gst-plugin-webrtc with livekit feature..."
cargo build --release --package gst-plugin-webrtc --features livekit

BUILT_LIB="target/release/libgstrswebrtc.dylib"

if [ ! -f "$BUILT_LIB" ]; then
    echo "Error: Built library not found at $BUILT_LIB"
    exit 1
fi

# Install to user directory instead of system directory
# This follows Homebrew's recommendation to avoid plugin deletion on upgrades
USER_PLUGIN_DIR="$HOME/.local/lib/gstreamer-1.0"
mkdir -p "$USER_PLUGIN_DIR"

echo "Installing to $USER_PLUGIN_DIR..."
cp "$BUILT_LIB" "$USER_PLUGIN_DIR/"

echo "=== Installation complete! ==="
echo ""
echo "To verify, run:"
echo "GST_PLUGIN_PATH=$USER_PLUGIN_DIR gst-inspect-1.0 livekitwebrtcsink"
echo ""
echo "To use in your project, run:"
echo "export GST_PLUGIN_PATH=$USER_PLUGIN_DIR"
echo "cargo run"
echo ""
echo "Or add this to your ~/.zshrc or ~/.bash_profile for permanent setup:"
echo "export GST_PLUGIN_PATH=\"\$HOME/.local/lib/gstreamer-1.0\""
