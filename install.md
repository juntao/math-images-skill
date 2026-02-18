# Math Images Skill — Installation Guide

Follow these steps to install the skill that generates PNG images from math equations in LaTeX/TeX and Markdown documents.

## Prerequisites

- `curl` or `wget` (for downloading)
- `unzip` (for extraction)
- `bash` (shell)
- `git` (for cloning)

## Quick Install (Recommended)

```bash
SKILL_DIR="${HOME}/.openclaw/skills/math-images"
mkdir -p "$SKILL_DIR"

# Clone the skill
git clone --depth 1 https://github.com/juntao/math-images-skill.git /tmp/math-images-repo
cp -r /tmp/math-images-repo/* "$SKILL_DIR"
cp /tmp/math-images-repo/.gitignore "$SKILL_DIR" 2>/dev/null || true
rm -rf /tmp/math-images-repo

# Download platform-specific binary
"${SKILL_DIR}/bootstrap.sh"
```

After installation, verify it works:

```bash
# Create a test file
cat > /tmp/test_math.tex << 'EOF'
The quadratic formula: $x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$
EOF

# Run math2img
mkdir -p /tmp/test_math_output
~/.openclaw/skills/math-images/scripts/math2img -i /tmp/test_math.tex -o /tmp/test_math_output
ls -la /tmp/test_math_output/equation_0001.png
```

## Manual Binary Installation

If automatic download fails, manually download the binary:

1. Go to https://github.com/juntao/math-images-skill/releases/latest
2. Download the zip for your platform:
   - `math-images-linux-x86_64.zip` (statically linked)
   - `math-images-linux-aarch64.zip` (statically linked)
   - `math-images-darwin-x86_64.zip`
   - `math-images-darwin-aarch64.zip`
   - `math-images-windows-x86_64.zip`
3. Extract to `~/.openclaw/skills/math-images/scripts/`
4. Make executable:
   ```bash
   chmod +x ~/.openclaw/skills/math-images/scripts/math2img
   ```

## Troubleshooting

### Download Failed
Check network connectivity:
```bash
curl -I "https://github.com/juntao/math-images-skill/releases/latest"
```

### Unsupported Platform
Check your platform:
```bash
echo "OS: $(uname -s), Arch: $(uname -m)"
```

Supported: Linux/macOS/Windows on x86_64/aarch64. Linux binaries are statically linked (musl) and work on any distro.
