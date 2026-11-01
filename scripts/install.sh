#!/bin/sh
set -e
cd "$(dirname "$0")/.."
./scripts/bundle.sh
DEST=/Applications
[ -w "$DEST" ] || DEST="$HOME/Applications"
mkdir -p "$DEST"
rm -rf "$DEST/Flip.app"
cp -R target/Flip.app "$DEST/Flip.app"
echo "installed $DEST/Flip.app"
