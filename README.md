# Math Images from Documents

Generate PNG images from math equations in LaTeX/TeX and Markdown documents. Built in Rust, designed for AI agents.

![Rust](https://img.shields.io/badge/rust-stable-orange) ![License: MIT](https://img.shields.io/badge/license-MIT-blue)

## Quick Start

This tool ships as an [OpenClaw](https://github.com/openclaw/openclaw) skill. Install it by telling your AI agent:

> Read https://raw.githubusercontent.com/juntao/math-images-skill/main/install.md and follow the instructions.

Once installed, send your agent a LaTeX or Markdown file and ask it to create images for the math equations. It will extract each equation, render it as a PNG image, and send the images back.

## What It Does

Give it a LaTeX or Markdown file containing math equations:

```latex
The famous equation $E = mc^2$ appears inline.

Display math:
$$\frac{d}{dx} \int_a^x f(t)\,dt = f(x)$$

A matrix:
\begin{pmatrix}
a & b \\
c & d
\end{pmatrix}
```

Get back publication-quality PNG images for each equation — ready to embed in blog posts, presentations, chat messages, or anywhere that doesn't support LaTeX rendering natively.

## Features

- **Single tool** — `math2img` extracts and renders all equations in one pass
- **LaTeX & Markdown** — Detects input format by file extension
- **Smart extraction** — Handles `$...$`, `$$...$$`, `\[...\]`, `\(...\)`, and environments (`equation`, `align`, `pmatrix`, `cases`, etc.)
- **Markdown-aware** — Skips math inside fenced code blocks and inline code
- **100+ LaTeX commands** — Greek letters, operators, fractions, square roots, matrices, subscripts, superscripts, arrows, accents, and more
- **Dark & light themes** — Dark (default) or light color scheme
- **STIX Two Math font** — Embedded font for consistent, high-quality rendering
- **Single binary** — No runtime dependencies, statically linked on Linux

## Installation

### Pre-built Binaries

Download from [Releases](https://github.com/juntao/math-images-skill/releases/latest):

| Platform | File |
|----------|------|
| Linux x86_64 (static) | `math-images-linux-x86_64.zip` |
| Linux aarch64 (static) | `math-images-linux-aarch64.zip` |
| macOS Intel | `math-images-darwin-x86_64.zip` |
| macOS Apple Silicon | `math-images-darwin-aarch64.zip` |
| Windows x86_64 | `math-images-windows-x86_64.zip` |

### Build from Source

```bash
cd math2img
cargo build --release
# Binary at: target/release/math2img
```

## Usage

```bash
# Basic usage (dark theme)
math2img -i document.tex -o output_dir/

# Light theme
math2img -i document.tex -o output_dir/ --theme light

# Markdown input
math2img -i document.md -o output_dir/

# Custom font size and scale
math2img -i document.tex -o output_dir/ --font-size 32 --scale 4.0
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `-i` | (required) | Input file path (`.tex`, `.md`, `.markdown`, `.mdx`) |
| `-o` | (required) | Output directory for PNG images |
| `--theme` | `dark` | Color theme (`dark` or `light`) |
| `--font-size` | `24` | Base font size in pixels |
| `--scale` | `3.0` | Rendering scale factor |

### Output

Equations are numbered sequentially: `equation_0001.png`, `equation_0002.png`, etc.

### Supported LaTeX Constructs

| Category | Examples |
|----------|---------|
| Inline math | `$...$`, `\(...\)` |
| Display math | `$$...$$`, `\[...\]` |
| Environments | `equation`, `align`, `gather`, `multline`, `displaymath`, `eqnarray` (and `*` variants) |
| Matrices | `pmatrix`, `bmatrix`, `vmatrix`, `Bmatrix`, `Vmatrix`, `cases` |
| Fractions | `\frac{a}{b}` |
| Roots | `\sqrt{x}`, `\sqrt[n]{x}` |
| Scripts | `x^2`, `x_i`, `x_i^2` |
| Greek | `\alpha`, `\beta`, `\gamma`, `\Gamma`, `\pi`, `\Pi`, ... |
| Operators | `\sum`, `\prod`, `\int`, `\lim`, `\sin`, `\cos`, ... |
| Relations | `=`, `\neq`, `\leq`, `\geq`, `\approx`, `\equiv`, ... |
| Arrows | `\to`, `\leftarrow`, `\Rightarrow`, `\leftrightarrow`, ... |
| Delimiters | `\left(`, `\right)`, `\left[`, `\right]`, `\left\{`, `\right\}` |
| Accents | `\hat`, `\bar`, `\vec`, `\dot`, `\tilde`, `\overline` |
| Spacing | `\,`, `\;`, `\quad`, `\qquad` |

## License

MIT
