#!/usr/bin/env bash
set -euo pipefail

REPO="auyer/protonup-rs"
BINARY="protonup-rsui"
VERSION="latest"
SYSTEM=0
PREFIX=""
NO_DESKTOP=0
UNINSTALL=0

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

info() { printf '%s\n' "$*"; }
ok()   { printf '%b%s%b\n' "${GREEN}" "$*" "${NC}"; }
err()  { printf '%b%s%b\n' "${RED}" "$*" "${NC}" >&2; }

usage() {
  cat <<'EOF'
Usage: install.sh [OPTIONS]

Install the ProtonUp-RS GUI (protonup-rsui) for users not using .deb or .rpm.

Options:
  -h, --help             Show this help message and exit
  -v, --version VERSION  Release tag to install (e.g. v1.0.0). Default: latest
      --system           Install system-wide (requires sudo/root)
      --prefix DIR       Install the binary into DIR instead of the default bin dir
      --no-desktop       Skip installing the desktop entry and icons
  -u, --uninstall        Remove a previously installed copy

Defaults (user install):
  binary       -> $HOME/.local/bin/protonup-rsui
  desktop file -> $HOME/.local/share/applications/protonup-rsui.desktop
  icons        -> $HOME/.local/share/icons/hicolor/.../apps/protonup-rsui.png
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)      usage; exit 0 ;;
    -v|--version)   VERSION="$2"; shift 2 ;;
    --system)       SYSTEM=1; shift ;;
    --prefix)       PREFIX="$2"; shift 2 ;;
    --no-desktop)   NO_DESKTOP=1; shift ;;
    -u|--uninstall) UNINSTALL=1; shift ;;
    *)              err "Unknown option: $1"; usage; exit 1 ;;
  esac
done

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)  printf 'amd64' ;;
    aarch64|arm64) printf 'arm64' ;;
    *)             return 1 ;;
  esac
}

download() {
  local url="$1" out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL --connect-timeout 60 -o "$out" "$url"
  elif command -v wget >/dev/null 2>&1; then
    wget -q --timeout=60 -O "$out" "$url"
  else
    err "error: neither curl nor wget is installed."
    exit 1
  fi
}

rewrite_exec() {
  local file="$1" bin="$2"
  local escaped="${bin//&/\\&}"
  sed -i "s|^Exec=.*|Exec=${escaped}|; s|^TryExec=.*|TryExec=${escaped}|" "$file"
}

add_to_path() {
  local dir="$1"
  case ":$PATH:" in
    *":$dir:"*) return 0 ;;
  esac

  for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [ -f "$rc" ] && ! grep -qF "$dir" "$rc"; then
      printf '\n# Added by the ProtonUp-RS installer\nexport PATH="$PATH:%s"\n' "$dir" >> "$rc"
    fi
  done
  ok "Added $dir to PATH in shell startup files (re-open your terminal to pick it up)."
}

ARCH="$(detect_arch)" || { err "error: unsupported architecture: $(uname -m)"; exit 1; }

if [ "$SYSTEM" = "1" ]; then
  BIN_DIR="${PREFIX:-/usr/local/bin}"
  APP_DIR="/usr/share/applications"
  ICON_DIR="/usr/share/icons/hicolor"
  PIXMAP_DIR="/usr/share/pixmaps"
else
  BIN_DIR="${PREFIX:-$HOME/.local/bin}"
  APP_DIR="$HOME/.local/share/applications"
  ICON_DIR="$HOME/.local/share/icons/hicolor"
  PIXMAP_DIR="$HOME/.local/share/pixmaps"
fi

DESKTOP_FILE="$APP_DIR/$BINARY.desktop"
ICON_256="$ICON_DIR/256x256/apps/$BINARY.png"
ICON_512="$ICON_DIR/512x512/apps/$BINARY.png"
PIXMAP_ICON="$PIXMAP_DIR/$BINARY.png"

if [ "$SYSTEM" = "1" ] && [ "$(id -u)" -ne 0 ]; then
  SUDO="sudo"
else
  SUDO=""
fi

if [ "$UNINSTALL" = "1" ]; then
  ok "Removing $BINARY..."
  $SUDO rm -f "$BIN_DIR/$BINARY" "$DESKTOP_FILE" "$ICON_256" "$ICON_512" "$PIXMAP_ICON"
  ok "Done."
  exit 0
fi

if [ "$VERSION" = "latest" ]; then
  REF="main"
  BIN_URL="https://github.com/${REPO}/releases/latest/download/${BINARY}-linux-${ARCH}.tar.gz"
else
  case "$VERSION" in
    v*) REF="$VERSION" ;;
    *)  REF="v$VERSION" ;;
  esac
  BIN_URL="https://github.com/${REPO}/releases/download/${REF}/${BINARY}-linux-${ARCH}.tar.gz"
fi

RAW_BASE="https://raw.githubusercontent.com/${REPO}/${REF}/protonup-rsui"
DESKTOP_URL="${RAW_BASE}/${BINARY}.desktop"
ICON_256_URL="${RAW_BASE}/assets/protonup-rs-logo_256.png"
ICON_512_URL="${RAW_BASE}/assets/protonup-rs-logo_512.png"
ICON_FULL_URL="${RAW_BASE}/assets/protonup-rs-logo.png"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

info "Downloading $BINARY ($ARCH) version '$VERSION'..."
download "$BIN_URL" "$tmp/$BINARY.tar.gz"
tar -xzf "$tmp/$BINARY.tar.gz" -C "$tmp"

info "Installing binary to $BIN_DIR..."
$SUDO mkdir -p "$BIN_DIR"
$SUDO install -m 755 "$tmp/$BINARY" "$BIN_DIR/$BINARY"

if [ "$NO_DESKTOP" = "0" ]; then
  info "Installing desktop entry and icons..."
  download "$DESKTOP_URL" "$tmp/$BINARY.desktop"
  rewrite_exec "$tmp/$BINARY.desktop" "$BIN_DIR/$BINARY"

  $SUDO mkdir -p "$APP_DIR" "$ICON_DIR/256x256/apps" "$ICON_DIR/512x512/apps" "$PIXMAP_DIR"
  $SUDO install -m 644 "$tmp/$BINARY.desktop" "$DESKTOP_FILE"

  download "$ICON_256_URL" "$tmp/icon_256.png"
  download "$ICON_512_URL" "$tmp/icon_512.png"
  download "$ICON_FULL_URL" "$tmp/icon_full.png"
  $SUDO install -m 644 "$tmp/icon_256.png" "$ICON_256"
  $SUDO install -m 644 "$tmp/icon_512.png" "$ICON_512"
  $SUDO install -m 644 "$tmp/icon_full.png" "$PIXMAP_ICON"
fi

if [ "$SYSTEM" = "0" ] && [ -z "$PREFIX" ]; then
  add_to_path "$BIN_DIR"
fi

ok "ProtonUp-RS installed successfully. You can launch it from your application menu."
