#!/usr/bin/env bash
set -euo pipefail

REPO="auyer/protonup-rs"
# Base URLs, overridable for testing (e.g. PROTONUP_RELEASES_BASE=file:///tmp/release).
RELEASES_BASE="${PROTONUP_RELEASES_BASE:-https://github.com/${REPO}}"
RAW_BASE="${PROTONUP_RAW_BASE:-https://raw.githubusercontent.com/${REPO}}"
VERSION="latest"
SYSTEM=0
PREFIX=""
NO_DESKTOP=0
UNINSTALL=0
ONLY_CLI=0
ONLY_GUI=0

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

info() { printf '%s\n' "$*"; }
ok()   { printf '%b%s%b\n' "${GREEN}" "$*" "${NC}"; }
err()  { printf '%b%s%b\n' "${RED}" "$*" "${NC}" >&2; }

usage() {
  cat <<'EOF'
Usage: install.sh [OPTIONS]

Install ProtonUp-RS. By default, installs both the CLI (protonup-rs) and the
GUI (protonup-rsui).

Options:
  -h, --help             Show this help message and exit
  -v, --version VERSION  Release tag to install (e.g. v1.0.0). Default: latest
      --cli              Install only the CLI (protonup-rs)
      --gui              Install only the GUI (protonup-rsui)
      --system           Install system-wide (requires sudo/root)
      --prefix DIR       Install binaries into DIR instead of the default bin dir
      --no-desktop       Skip installing the GUI desktop entry and icons
  -u, --uninstall        Remove a previously installed copy

Defaults (user install):
  binaries     -> $HOME/.local/bin/
  desktop file -> $HOME/.local/share/applications/protonup-rsui.desktop
  icons        -> $HOME/.local/share/icons/hicolor/.../apps/protonup-rsui.png
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help)      usage; exit 0 ;;
    -v|--version)   VERSION="$2"; shift 2 ;;
    --cli)          ONLY_CLI=1; shift ;;
    --gui)          ONLY_GUI=1; shift ;;
    --system)       SYSTEM=1; shift ;;
    --prefix)       PREFIX="$2"; shift 2 ;;
    --no-desktop)   NO_DESKTOP=1; shift ;;
    -u|--uninstall) UNINSTALL=1; shift ;;
    *)              err "Unknown option: $1"; usage; exit 1 ;;
  esac
done

# Resolve which apps to install (default: both).
if [ "$ONLY_CLI" = "1" ] && [ "$ONLY_GUI" = "1" ]; then
  INSTALL_CLI=1; INSTALL_GUI=1
elif [ "$ONLY_CLI" = "1" ]; then
  INSTALL_CLI=1; INSTALL_GUI=0
elif [ "$ONLY_GUI" = "1" ]; then
  INSTALL_CLI=0; INSTALL_GUI=1
else
  INSTALL_CLI=1; INSTALL_GUI=1
fi

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
      # shellcheck disable=SC2016  # the literal $PATH is intentional (written to the rc file)
      printf '\n# Added by the ProtonUp-RS installer\nexport PATH="$PATH:%s"\n' "$dir" >> "$rc"
    fi
  done
  ok "Added $dir to PATH in shell startup files (re-open your terminal to pick it up)."
}

bin_url() {
  local bin="$1"
  if [ "$VERSION" = "latest" ]; then
    printf '%s/releases/latest/download/%s-linux-%s.tar.gz' "$RELEASES_BASE" "$bin" "$ARCH"
  else
    printf '%s/releases/download/%s/%s-linux-%s.tar.gz' "$RELEASES_BASE" "$REF" "$bin" "$ARCH"
  fi
}

install_binary() {
  local bin="$1" url
  url="$(bin_url "$bin")"

  info "Downloading $bin ($ARCH) version '$VERSION'..."
  download "$url" "$tmp/$bin.tar.gz"
  tar -xzf "$tmp/$bin.tar.gz" -C "$tmp"

  info "Installing binary to $BIN_DIR..."
  $SUDO mkdir -p "$BIN_DIR"
  $SUDO install -m 755 "$tmp/$bin" "$BIN_DIR/$bin"
}

install_gui_desktop() {
  local bin="protonup-rsui"
  local raw_base="${RAW_BASE}/${REF}/protonup-rsui"
  local desktop_file="$APP_DIR/$bin.desktop"
  local icon_256="$ICON_DIR/256x256/apps/$bin.png"
  local icon_512="$ICON_DIR/512x512/apps/$bin.png"
  local pixmap_icon="$PIXMAP_DIR/$bin.png"

  info "Installing desktop entry and icons..."
  download "${raw_base}/${bin}.desktop" "$tmp/$bin.desktop"
  rewrite_exec "$tmp/$bin.desktop" "$BIN_DIR/$bin"

  $SUDO mkdir -p "$APP_DIR" "$ICON_DIR/256x256/apps" "$ICON_DIR/512x512/apps" "$PIXMAP_DIR"
  $SUDO install -m 644 "$tmp/$bin.desktop" "$desktop_file"

  download "${raw_base}/assets/protonup-rs-logo_256.png" "$tmp/icon_256.png"
  download "${raw_base}/assets/protonup-rs-logo_512.png" "$tmp/icon_512.png"
  download "${raw_base}/assets/protonup-rs-logo.png" "$tmp/icon_full.png"
  $SUDO install -m 644 "$tmp/icon_256.png" "$icon_256"
  $SUDO install -m 644 "$tmp/icon_512.png" "$icon_512"
  $SUDO install -m 644 "$tmp/icon_full.png" "$pixmap_icon"
}

ARCH="$(detect_arch)" || { err "error: unsupported architecture: $(uname -m)"; exit 1; }

if [ "$VERSION" = "latest" ]; then
  REF="main"
else
  case "$VERSION" in
    v*) REF="$VERSION" ;;
    *)  REF="v$VERSION" ;;
  esac
fi

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

if [ "$SYSTEM" = "1" ] && [ "$(id -u)" -ne 0 ]; then
  SUDO="sudo"
else
  SUDO=""
fi

if [ "$UNINSTALL" = "1" ]; then
  if [ "$INSTALL_CLI" = "1" ]; then
    ok "Removing protonup-rs..."
    $SUDO rm -f "$BIN_DIR/protonup-rs"
  fi
  if [ "$INSTALL_GUI" = "1" ]; then
    ok "Removing protonup-rsui..."
    $SUDO rm -f \
      "$BIN_DIR/protonup-rsui" \
      "$APP_DIR/protonup-rsui.desktop" \
      "$ICON_DIR/256x256/apps/protonup-rsui.png" \
      "$ICON_DIR/512x512/apps/protonup-rsui.png" \
      "$PIXMAP_DIR/protonup-rsui.png"
  fi
  ok "Done."
  exit 0
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if [ "$INSTALL_CLI" = "1" ]; then
  install_binary "protonup-rs"
fi

if [ "$INSTALL_GUI" = "1" ]; then
  install_binary "protonup-rsui"
  if [ "$NO_DESKTOP" = "0" ]; then
    install_gui_desktop
  fi
fi

if [ "$SYSTEM" = "0" ] && [ -z "$PREFIX" ]; then
  add_to_path "$BIN_DIR"
fi

if [ "$INSTALL_CLI" = "1" ] && [ "$INSTALL_GUI" = "1" ]; then
  ok "ProtonUp-RS installed successfully. Run 'protonup-rs' in a terminal or launch 'protonup-rsui' from your application menu."
elif [ "$INSTALL_CLI" = "1" ]; then
  ok "ProtonUp-RS CLI installed successfully. Run 'protonup-rs' in a terminal."
else
  ok "ProtonUp-RS GUI installed successfully. Launch 'protonup-rsui' from your application menu."
fi
