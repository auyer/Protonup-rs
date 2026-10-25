#!/usr/bin/env bats

# Tests for scripts/install.sh.
#
# These tests never touch the network: a fake `curl` is placed on PATH that
# serves `file://` URLs from a local fixture tree, and PROTONUP_RELEASES_BASE /
# PROTONUP_RAW_BASE point the installer at that tree.

INSTALLER="$(cd "$(dirname "$BATS_TEST_FILENAME")/.." && pwd)/install.sh"

setup() {
  TEST_ROOT="$(mktemp -d)"
  FIXTURE="$TEST_ROOT/fixture"
  BIN_DIR="$TEST_ROOT/bin"
  FAKEBIN="$TEST_ROOT/fakebin"

  mkdir -p "$FIXTURE" "$BIN_DIR" "$TEST_ROOT/home" "$FAKEBIN"

  # Fake curl: copies file:// URLs to the -o destination.
  cat > "$FAKEBIN/curl" <<'EOF'
#!/usr/bin/env bash
out=""
url=""
while [ $# -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    --connect-timeout) shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done
src="${url#file://}"
if [ -z "$out" ]; then
  echo "fake-curl: missing -o destination" >&2
  exit 2
fi
if [ ! -f "$src" ]; then
  echo "fake-curl: no such file: $src" >&2
  exit 37
fi
cp "$src" "$out"
EOF
  chmod +x "$FAKEBIN/curl"

  export PATH="$FAKEBIN:$PATH"
  export HOME="$TEST_ROOT/home"
  export PROTONUP_RELEASES_BASE="file://$FIXTURE"
  export PROTONUP_RAW_BASE="file://$FIXTURE"

  make_fixture
}

teardown() {
  rm -rf "$TEST_ROOT"
}

make_fixture() {
  local arch bin
  for arch in amd64 arm64; do
    mkdir -p "$FIXTURE/releases/latest/download" "$FIXTURE/releases/download/v1.0.0"
    for bin in protonup-rs protonup-rsui; do
      (
        cd "$TEST_ROOT" &&
          printf '#!/bin/sh\necho %s\n' "$bin" > "$bin" &&
          chmod +x "$bin" &&
          tar -czf "$FIXTURE/releases/latest/download/${bin}-linux-${arch}.tar.gz" "$bin" &&
          tar -czf "$FIXTURE/releases/download/v1.0.0/${bin}-linux-${arch}.tar.gz" "$bin"
      )
    done
  done

  mkdir -p "$FIXTURE/main/protonup-rsui/assets"
  cat > "$FIXTURE/main/protonup-rsui/protonup-rsui.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=ProtonUp-RS
Exec=protonup-rsui
TryExec=protonup-rsui
Icon=protonup-rsui
Terminal=false
Categories=Utility;
EOF
  printf '256x256\n' > "$FIXTURE/main/protonup-rsui/assets/protonup-rs-logo_256.png"
  printf '512x512\n' > "$FIXTURE/main/protonup-rsui/assets/protonup-rs-logo_512.png"
  printf 'full\n'      > "$FIXTURE/main/protonup-rsui/assets/protonup-rs-logo.png"
}

@test "prints help and exits 0" {
  run bash "$INSTALLER" --help
  [ "$status" -eq 0 ]
  [[ "$output" == *"--cli"* ]]
  [[ "$output" == *"--gui"* ]]
  [[ "$output" == *"--uninstall"* ]]
}

@test "rejects unknown options" {
  run bash "$INSTALLER" --bogus
  [ "$status" -ne 0 ]
}

@test "installs both apps by default" {
  run bash "$INSTALLER" --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  [ -x "$BIN_DIR/protonup-rs" ]
  [ -x "$BIN_DIR/protonup-rsui" ]
  [ -f "$HOME/.local/share/applications/protonup-rsui.desktop" ]
  [ -f "$HOME/.local/share/icons/hicolor/256x256/apps/protonup-rsui.png" ]
  [ -f "$HOME/.local/share/icons/hicolor/512x512/apps/protonup-rsui.png" ]
  [ -f "$HOME/.local/share/pixmaps/protonup-rsui.png" ]
}

@test "--cli installs only the CLI" {
  run bash "$INSTALLER" --cli --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  [ -x "$BIN_DIR/protonup-rs" ]
  [ ! -e "$BIN_DIR/protonup-rsui" ]
  [ ! -e "$HOME/.local/share/applications/protonup-rsui.desktop" ]
}

@test "--gui installs only the GUI plus desktop entry and icons" {
  run bash "$INSTALLER" --gui --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  [ ! -e "$BIN_DIR/protonup-rs" ]
  [ -x "$BIN_DIR/protonup-rsui" ]
  [ -f "$HOME/.local/share/applications/protonup-rsui.desktop" ]
  [ -f "$HOME/.local/share/icons/hicolor/256x256/apps/protonup-rsui.png" ]
}

@test "--no-desktop skips the desktop entry and icons" {
  run bash "$INSTALLER" --gui --no-desktop --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  [ -x "$BIN_DIR/protonup-rsui" ]
  [ ! -e "$HOME/.local/share/applications/protonup-rsui.desktop" ]
  [ ! -e "$HOME/.local/share/icons/hicolor/256x256/apps/protonup-rsui.png" ]
}

@test "rewrites Exec and TryExec to the installed binary path" {
  run bash "$INSTALLER" --gui --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  local desktop="$HOME/.local/share/applications/protonup-rsui.desktop"
  grep -q "^Exec=$BIN_DIR/protonup-rsui$" "$desktop"
  grep -q "^TryExec=$BIN_DIR/protonup-rsui$" "$desktop"
}

@test "installs a specific version" {
  run bash "$INSTALLER" --version v1.0.0 --cli --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  [ -x "$BIN_DIR/protonup-rs" ]
  [ ! -e "$BIN_DIR/protonup-rsui" ]
}

@test "--uninstall --cli removes only the CLI" {
  run bash "$INSTALLER" --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  run bash "$INSTALLER" --uninstall --cli --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  [ ! -e "$BIN_DIR/protonup-rs" ]
  [ -x "$BIN_DIR/protonup-rsui" ]
  [ -f "$HOME/.local/share/applications/protonup-rsui.desktop" ]
}

@test "--uninstall --gui removes the GUI and its desktop entry" {
  run bash "$INSTALLER" --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  run bash "$INSTALLER" --uninstall --gui --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  [ -x "$BIN_DIR/protonup-rs" ]
  [ ! -e "$BIN_DIR/protonup-rsui" ]
  [ ! -e "$HOME/.local/share/applications/protonup-rsui.desktop" ]
}

@test "--uninstall removes both by default" {
  run bash "$INSTALLER" --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  run bash "$INSTALLER" --uninstall --prefix "$BIN_DIR"
  [ "$status" -eq 0 ]

  [ ! -e "$BIN_DIR/protonup-rs" ]
  [ ! -e "$BIN_DIR/protonup-rsui" ]
  [ ! -e "$HOME/.local/share/applications/protonup-rsui.desktop" ]
}
