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

get_download_url() {
    local platform="$1"
    local artifact_name="math-images-${platform}.zip"
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    local download_url

    if command -v curl &>/dev/null; then
        download_url=$(curl -sL "$api_url" | grep -o "https://github.com/${REPO}/releases/download/[^\"]*${artifact_name}" | head -1)
    elif command -v wget &>/dev/null; then
        download_url=$(wget -qO- "$api_url" | grep -o "https://github.com/${REPO}/releases/download/[^\"]*${artifact_name}" | head -1)
    else
        echo "Error: Neither curl nor wget found." >&2
        exit 1
    fi

    if [ -z "$download_url" ]; then
        echo "Error: Could not find release for platform ${platform}" >&2
        echo "Check https://github.com/${REPO}/releases for available downloads." >&2
        exit 1
    fi

    echo "$download_url"
}

download_binary() {
    local platform="$1"
    local url="$2"
    local temp_dir

    echo "Downloading math2img for ${platform}..." >&2

    mkdir -p "${SCRIPTS_DIR}"

    temp_dir=$(mktemp -d)
    local zip_file="${temp_dir}/math-images-${platform}.zip"

    echo "Fetching from: ${url}" >&2
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
        exit 1
    fi

    if [[ "$(uname -s)" != MINGW* ]] && [[ "$(uname -s)" != MSYS* ]] && [[ "$(uname -s)" != CYGWIN* ]]; then
        chmod +x "${SCRIPTS_DIR}/math2img"
    fi

    rm -rf "$temp_dir"

    echo "math2img installed to ${SCRIPTS_DIR}" >&2
}

main() {
    local platform
    platform=$(detect_platform)
    echo "Detected platform: ${platform}" >&2

    local download_url
    download_url=$(get_download_url "$platform")

    download_binary "$platform" "$download_url"

    echo "" >&2
    echo "Installed:" >&2
    ls -1 "${SCRIPTS_DIR}" | grep -v '^\.' >&2
}

main "$@"
