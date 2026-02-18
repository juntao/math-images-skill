---
name: math-equation-images
description: Generate PNG images from math equations in a LaTeX/TeX or Markdown document. Triggered when user says they need images for math equations, formulae, or matrices. The document can be provided inline in the message body or as a Telegram file attachment. Generated images are sent back as Telegram messages. Supports dark (default) and light themes — user can request "light" or "dark" style.
---

# Math Equation Images

Extract math equations from LaTeX/TeX or Markdown documents and render each as a PNG image.

## Binary

* `{baseDir}/scripts/math2img` — Renders math equations to PNG images.

## Workflow

### 1. Extract Document

- **Inline**: Extract LaTeX/Markdown text from the message body (everything after the request)
- **File attachment**: Read the file from the Telegram media path

### 2. Save Document

Write the content to a temp file with the appropriate extension:
- `.tex` for LaTeX/TeX content
- `.md` for Markdown content

```bash
cat > /tmp/math_input.tex << 'DOCEOF'
<document content here>
DOCEOF
```

### 3. Render Math Images

Run math2img on the document:
```bash
mkdir -p /tmp/math_output
{baseDir}/scripts/math2img -i /tmp/math_input.tex -o /tmp/math_output
```

Options:
- `--theme <dark|light>` — Color theme. Default: `dark`
- `--font-size <px>` — Font size. Default: `24`
- `--scale <factor>` — Scale factor. Default: `3.0`

**Style selection:** If the user requests "light" style/theme, use `--theme light`. Default is dark. The user may also say "light mode", "white background", "light theme", etc.

### 4. Send Images via Telegram

Copy rendered PNGs to the allowed media directory, then send via the `message` tool:

```bash
cp /tmp/math_output/equation_0001.png ~/.openclaw/media/inbound/equation_0001.png
```

Send with:
- `action: send`
- `filePath: ~/.openclaw/media/inbound/equation_0001.png`
- `caption: "Equation 1"`

Send all images in order. If no equations found, tell the user.
