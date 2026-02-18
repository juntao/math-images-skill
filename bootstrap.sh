#!/bin/bash
# Bootstrap script for math-equation-images skill
# Downloads and installs platform-specific math2img binary

set -e

REPO="juntao/math-images-skill"
SKILL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="${SKILL_DIR}/scripts"

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="darwin" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *)
            echo "Error: Unsupported operating system: $(uname -s)" >&2
            exit 1
            ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)
            echo "Error: Unsupported architecture: $(uname -m)" >&2
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

download_and_extract() {
    local platform="$1"
    local artifact_prefix="$2"
    local binary_name="$3"
    local url

    local artifact_name="${artifact_prefix}-${platform}.zip"
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"

    if command -v curl &>/dev/null; then
        url=$(curl -sL "$api_url" | grep -o "https://github.com/${REPO}/releases/download/[^\"]*${artifact_name}" | head -1)
    elif command -v wget &>/dev/null; then
        url=$(wget -qO- "$api_url" | grep -o "https://github.com/${REPO}/releases/download/[^\"]*${artifact_name}" | head -1)
    fi

    if [ -z "$url" ]; then
        echo "Warning: Could not find release for ${binary_name} (${platform}), skipping." >&2
        return 1
    fi

    echo "Downloading ${binary_name} for ${platform}..." >&2
    echo "Fetching from: ${url}" >&2

    local temp_dir
    temp_dir=$(mktemp -d)
    local zip_file="${temp_dir}/${artifact_name}"

    if command -v curl &>/dev/null; then
        curl -sL -o "$zip_file" "$url"
    else
        wget -q -O "$zip_file" "$url"
    fi

    echo "Extracting binary..." >&2
    if command -v unzip &>/dev/null; then
        unzip -q -o "$zip_file" -d "${SCRIPTS_DIR}"
    else
        echo "Error: unzip not found." >&2
        rm -rf "$temp_dir"
        return 1
    fi

    if [[ "$(uname -s)" != MINGW* ]] && [[ "$(uname -s)" != MSYS* ]] && [[ "$(uname -s)" != CYGWIN* ]]; then
        chmod +x "${SCRIPTS_DIR}/${binary_name}" 2>/dev/null || true
    fi

    rm -rf "$temp_dir"
    echo "${binary_name} installed to ${SCRIPTS_DIR}" >&2
    return 0
}

main() {
    local platform
    platform=$(detect_platform)
    echo "Detected platform: ${platform}" >&2

    mkdir -p "${SCRIPTS_DIR}"

    # Install math2img (pure Rust, primary)
    download_and_extract "$platform" "math-images" "math2img"

    # Install math2img-tectonic (TeX backend, optional use)
    download_and_extract "$platform" "math-images-tectonic" "math2img-tectonic" || true

    echo "" >&2
    echo "Installed:" >&2
    ls -1 "${SCRIPTS_DIR}" | grep -v '^\.' >&2
}

main "$@"
