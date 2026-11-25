#!/bin/sh
set -e
cd "$(dirname "$0")/.."

case "$(uname -s)" in
Darwin)
    ./scripts/bundle.sh
    DEST=/Applications
    [ -w "$DEST" ] || DEST="$HOME/Applications"
    mkdir -p "$DEST"
    rm -rf "$DEST/Flip.app"
    cp -R target/Flip.app "$DEST/Flip.app"
    echo "installed $DEST/Flip.app"
    ;;
Linux)
    cargo build --release
    BIN="$HOME/.local/bin"
    APPS="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
    mkdir -p "$BIN" "$APPS"
    install -m 755 target/release/flip "$BIN/flip"
    cat > "$APPS/flip.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=Flip
GenericName=Sampler
Comment=Sampler and drum machine
Exec=$BIN/flip
Icon=audio-x-generic
Terminal=false
StartupWMClass=flip
Categories=AudioVideo;Audio;Music;
Keywords=sampler;drum;beat;pads;
DESKTOP
    command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" || true
    echo "installed $BIN/flip and the Flip menu entry"
    case ":$PATH:" in
    *":$BIN:"*) ;;
    *) echo "note: $BIN is not in PATH. Start Flip from the app menu, or add it to PATH." ;;
    esac
    ;;
*)
    echo "unsupported system: $(uname -s)" >&2
    exit 1
    ;;
esac
