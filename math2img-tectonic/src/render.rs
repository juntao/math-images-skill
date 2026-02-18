use anyhow::{bail, Context, Result};
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    fn text_color(&self) -> &str {
        match self {
            Theme::Dark => "0.753,0.773,0.808",
            Theme::Light => "0,0,0",
        }
    }

    pub fn bg_color(&self) -> [u8; 3] {
        match self {
            Theme::Dark => [43, 48, 59],
            Theme::Light => [255, 255, 255],
        }
    }

    fn page_color(&self) -> &str {
        match self {
            Theme::Dark => "0.169,0.188,0.231",
            Theme::Light => "1,1,1",
        }
    }
}

/// Wrap a LaTeX math snippet in a standalone document for rendering.
fn wrap_equation(content: &str, is_display: bool, theme: &Theme, font_size: f32) -> String {
    let size_cmd = match font_size as u32 {
        0..=9 => "\\scriptsize",
        10..=11 => "\\small",
        12 => "\\normalsize",
        13..=14 => "\\large",
        15..=17 => "\\Large",
        18..=20 => "\\LARGE",
        21..=24 => "\\huge",
        _ => "\\Huge",
    };

    // Strip equation numbering — replace numbered environments with starred variants
    let content = content
        .replace("\\begin{equation}", "\\begin{equation*}")
        .replace("\\end{equation}", "\\end{equation*}")
        .replace("\\begin{align}", "\\begin{align*}")
        .replace("\\end{align}", "\\end{align*}")
        .replace("\\begin{gather}", "\\begin{gather*}")
        .replace("\\end{gather}", "\\end{gather*}");

    let math = if is_display {
        if content.contains("\\begin{") {
            content.to_string()
        } else {
            format!("\\[{}\\]", content)
        }
    } else {
        format!("${}$", content)
    };

    format!(
        r#"\documentclass[preview,border=12pt,varwidth=80cm]{{standalone}}
\usepackage{{amsmath}}
\usepackage{{amssymb}}
\usepackage{{amsfonts}}
\usepackage{{xcolor}}
\pagecolor[rgb]{{{page_color}}}
\color[rgb]{{{text_color}}}
\pagestyle{{empty}}
\begin{{document}}
{size_cmd}
{math}
\end{{document}}"#,
        page_color = theme.page_color(),
        text_color = theme.text_color(),
        size_cmd = size_cmd,
        math = math,
    )
}

/// Render equation: LaTeX → PDF (tectonic) → PNG (pdftoppm/sips/mutool)
pub fn render_equation(
    content: &str,
    is_display: bool,
    theme: &Theme,
    font_size: f32,
    scale: f32,
    output: &Path,
) -> Result<()> {
    let latex_src = wrap_equation(content, is_display, theme, font_size);
    let tmp = tempfile::tempdir()?;
    let tex_path = tmp.path().join("eq.tex");
    std::fs::write(&tex_path, &latex_src)?;

    // Step 1: tectonic → PDF
    let tec = Command::new("tectonic")
        .args(["-X", "compile", "--untrusted",
               "-o", &tmp.path().to_string_lossy(),
               &tex_path.to_string_lossy().to_string()])
        .output()
        .context("Failed to run tectonic. Is it installed?")?;

    if !tec.status.success() {
        let stderr = String::from_utf8_lossy(&tec.stderr);
        bail!("tectonic failed: {}", stderr.lines().last().unwrap_or("unknown error"));
    }

    let pdf_path = tmp.path().join("eq.pdf");
    if !pdf_path.exists() {
        bail!("tectonic produced no PDF output");
    }

    // Step 2: PDF → PNG
    let dpi = (72.0 * scale) as u32;
    let png_stem = tmp.path().join("eq");

    // Try pdftoppm first (poppler — most reliable)
    let r = Command::new("pdftoppm")
        .args(["-png", "-r", &dpi.to_string(), "-singlefile",
               &pdf_path.to_string_lossy().to_string(),
               &png_stem.to_string_lossy().to_string()])
        .output();

    let png_path = tmp.path().join("eq.png");
    let converted = match r {
        Ok(o) if o.status.success() && png_path.exists() => true,
        _ => {
            // Fallback: sips (macOS)
            let r2 = Command::new("sips")
                .args(["-s", "format", "png",
                       &pdf_path.to_string_lossy().to_string(),
                       "--out", &png_path.to_string_lossy().to_string()])
                .output();
            match r2 {
                Ok(o2) if o2.status.success() && png_path.exists() => true,
                _ => {
                    // Fallback: mutool
                    let r3 = Command::new("mutool")
                        .args(["draw", "-o", &png_path.to_string_lossy().to_string(),
                               "-r", &dpi.to_string(),
                               &pdf_path.to_string_lossy().to_string()])
                        .output();
                    matches!(r3, Ok(o3) if o3.status.success() && png_path.exists())
                }
            }
        }
    };

    if !converted {
        bail!("PDF→PNG conversion failed. Install poppler (pdftoppm), or ensure sips/mutool is available.");
    }

    // Step 3: Auto-crop the PNG to content bounds
    let trimmed = autocrop_png(&png_path, theme)?;
    std::fs::write(output, &trimmed).context("Failed to write trimmed PNG")?;
    Ok(())
}

/// Read a PNG, find the bounding box of non-background pixels, crop with padding, re-encode.
fn autocrop_png(path: &Path, theme: &Theme) -> Result<Vec<u8>> {
    let file = BufReader::new(std::fs::File::open(path)?);
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info()?;
    let (ow, oh) = reader.info().size();
    let channels_count = reader.info().color_type.samples();
    let buf_size = (ow as usize) * (oh as usize) * channels_count;
    let mut buf = vec![0u8; buf_size];
    let info = reader.next_frame(&mut buf)?;
    buf.truncate(info.buffer_size());
    let width = info.width as usize;
    let height = info.height as usize;
    let channels = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        _ => bail!("Unsupported PNG color type: {:?}", info.color_type),
    };

    let bg = theme.bg_color();
    let tolerance = 30u8; // allow slight antialiasing differences

    let is_bg = |idx: usize| -> bool {
        let r = buf[idx];
        let g = buf[idx + 1];
        let b = buf[idx + 2];
        r.abs_diff(bg[0]) <= tolerance
            && g.abs_diff(bg[1]) <= tolerance
            && b.abs_diff(bg[2]) <= tolerance
    };

    // Find content bounds
    let mut min_x = width;
    let mut max_x = 0usize;
    let mut min_y = height;
    let mut max_y = 0usize;

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * channels;
            if !is_bg(idx) {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }

    if min_x > max_x || min_y > max_y {
        // All background — return original
        return Ok(std::fs::read(path)?);
    }

    // Add padding — ensure minimum height for reasonable aspect ratio
    let pad = 16usize;
    let cx0 = min_x.saturating_sub(pad);
    let cy0 = min_y.saturating_sub(pad);
    let cx1 = (max_x + pad + 1).min(width);
    let cy1 = (max_y + pad + 1).min(height);
    let cw = cx1 - cx0;
    let ch = cy1 - cy0;

    // Ensure minimum height for sane aspect ratio (Telegram rejects extreme aspect ratios)
    let min_height = cw / 8; // max 8:1 aspect ratio
    let (cy0, cy1, ch) = if ch < min_height {
        let extra = min_height - ch;
        let new_cy0 = cy0.saturating_sub(extra / 2);
        let new_cy1 = (cy1 + extra / 2).min(height);
        let new_ch = new_cy1 - new_cy0;
        (new_cy0, new_cy1, new_ch)
    } else {
        (cy0, cy1, ch)
    };

    // Build cropped buffer
    let mut cropped = Vec::with_capacity(cw * ch * channels);
    for y in cy0..cy1 {
        let row_start = (y * width + cx0) * channels;
        let row_end = (y * width + cx1) * channels;
        cropped.extend_from_slice(&buf[row_start..row_end]);
    }

    // Encode
    let mut out = Vec::new();
    {
        let w = BufWriter::new(&mut out);
        let mut encoder = png::Encoder::new(w, cw as u32, ch as u32);
        encoder.set_color(info.color_type);
        encoder.set_depth(info.bit_depth);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&cropped)?;
    }
    Ok(out)
}
