#!/usr/bin/env bash
# Build (and optionally install) meetily-plus, this fork, on Windows from Git Bash.
#   ./build-plus.sh            -> builds the installer
#   ./build-plus.sh --install  -> builds and silently installs to %LOCALAPPDATA%\meetily-plus
# Close meetily-plus before installing. To pick up upstream changes first:
#   git fetch upstream && git merge upstream/main
set -euo pipefail
cd "$(dirname "$0")"

export PATH="$HOME/.cargo/bin:/c/Program Files/CMake/bin:$PATH"
# whisper-rs-sys 0.11 (bindgen 0.69) generates empty structs with libclang 19+, so pin libclang 18
# (libclang.dll from the PyPI "libclang==18.1.1" wheel).
export LIBCLANG_PATH="${LIBCLANG_PATH:-C:/Users/LMAhm/tools/libclang-18/libclang-18.1.1.data/platlib/clang/native}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-C:/cargo-target/meetily}"

# Local LLM sidecar must exist before the app build. Built with Vulkan so summaries run on the
# GPU (CPU-only took ~13 min for an 11k-token transcript); needs the Vulkan SDK in C:\VulkanSDK.
cmd //c "$(cygpath -w "$PWD/build-llama-helper-vulkan.bat")"
cp "$CARGO_TARGET_DIR/release/llama-helper.exe" src-tauri/binaries/llama-helper-x86_64-pc-windows-msvc.exe

npx -y pnpm@9.15.9 install --frozen-lockfile
# signCommand is upstream's DigiCert signing; without their cert Tauri fails looking for SignTool
npx -y pnpm@9.15.9 tauri build --bundles nsis --config '{"bundle":{"windows":{"signCommand":null}}}'

SETUP=$(ls -t "$CARGO_TARGET_DIR"/release/bundle/nsis/*-setup.exe | head -1)
echo "Installer: $SETUP"
if [[ "${1:-}" == "--install" ]]; then
  "$SETUP" //S
  echo "Installed to $LOCALAPPDATA\\meetily-plus"
fi
