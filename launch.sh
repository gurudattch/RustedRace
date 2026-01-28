#!/bin/bash

# RustedRace Universal Launcher
# Works on any Linux system

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Set executable path
EXECUTABLE="$SCRIPT_DIR/target/release/rustedrace"

# Check if executable exists
if [ ! -f "$EXECUTABLE" ]; then
    echo "Error: rustedrace executable not found at $EXECUTABLE"
    echo "Please run 'cargo build --release' first"
    exit 1
fi

# Make executable if not already
chmod +x "$EXECUTABLE"

# Create desktop entry dynamically
DESKTOP_FILE="$SCRIPT_DIR/rustedrace.desktop"
cat > "$DESKTOP_FILE" << EOF
[Desktop Entry]
Name=RustedRace
Comment=Race Condition Vulnerability Explorer
Exec=$EXECUTABLE
Icon=$SCRIPT_DIR/src/rustedrace.png
Terminal=false
Type=Application
Categories=Development;Security;
EOF

# Make desktop file executable
chmod +x "$DESKTOP_FILE"

# Install desktop entry for current user
mkdir -p ~/.local/share/applications
cp "$DESKTOP_FILE" ~/.local/share/applications/

# Launch the application
echo "Starting RustedRace..."
"$EXECUTABLE" &

echo "RustedRace launched successfully!"
echo "Desktop entry installed to ~/.local/share/applications/"
