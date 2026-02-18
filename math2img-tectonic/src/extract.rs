use regex_lite::Regex;

#[derive(Debug, Clone)]
pub struct Equation {
    pub content: String,
    pub is_display: bool,
    pub start: usize,
    pub end: usize,
}

fn overlaps(used: &[(usize, usize)], start: usize, end: usize) -> bool {
    used.iter().any(|&(s, e)| start < e && end > s)
}

/// Extract math equations from a LaTeX/TeX document.
pub fn extract_from_latex(content: &str) -> Vec<Equation> {
    let mut equations = Vec::new();
    let mut used: Vec<(usize, usize)> = Vec::new();

    // Phase 1: Display math environments
    let env_re = Regex::new(
        r"(?s)\\begin\{(equation\*?|align\*?|gather\*?|multline\*?|displaymath|flalign\*?|eqnarray\*?|pmatrix|bmatrix|vmatrix|Bmatrix|Vmatrix|cases)\}(.*?)\\end\{(equation\*?|align\*?|gather\*?|multline\*?|displaymath|flalign\*?|eqnarray\*?|pmatrix|bmatrix|vmatrix|Bmatrix|Vmatrix|cases)\}"
    ).unwrap();

    for cap in env_re.captures_iter(content) {
        let m = cap.get(0).unwrap();
        // Verify opening and closing env names match
        if cap.get(1).map(|m| m.as_str()) == cap.get(3).map(|m| m.as_str()) {
            if !overlaps(&used, m.start(), m.end()) {
                used.push((m.start(), m.end()));
                equations.push(Equation {
                    content: m.as_str().to_string(),
                    is_display: true,
                    start: m.start(),
                    end: m.end(),
                });
            }
        }
    }

    // $$...$$ (display math)
    let dd_re = Regex::new(r"(?s)\$\$(.*?)\$\$").unwrap();
    for cap in dd_re.captures_iter(content) {
        let m = cap.get(0).unwrap();
        if !overlaps(&used, m.start(), m.end()) {
            used.push((m.start(), m.end()));
            equations.push(Equation {
                content: cap.get(1).unwrap().as_str().trim().to_string(),
                is_display: true,
                start: m.start(),
                end: m.end(),
            });
        }
    }

    // \[...\] (display math)
    let bracket_re = Regex::new(r"(?s)\\\[(.*?)\\\]").unwrap();
    for cap in bracket_re.captures_iter(content) {
        let m = cap.get(0).unwrap();
        if !overlaps(&used, m.start(), m.end()) {
            used.push((m.start(), m.end()));
            equations.push(Equation {
                content: cap.get(1).unwrap().as_str().trim().to_string(),
                is_display: true,
                start: m.start(),
                end: m.end(),
            });
        }
    }

    // Phase 2: Inline math

    // \(...\) (inline math)
    let paren_re = Regex::new(r"(?s)\\\((.*?)\\\)").unwrap();
    for cap in paren_re.captures_iter(content) {
        let m = cap.get(0).unwrap();
        if !overlaps(&used, m.start(), m.end()) {
            used.push((m.start(), m.end()));
            equations.push(Equation {
                content: cap.get(1).unwrap().as_str().trim().to_string(),
                is_display: false,
                start: m.start(),
                end: m.end(),
            });
        }
    }

    // $...$ (inline, not $$) — byte-level state machine
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'$' { i += 2; continue; }
            if i > 0 && bytes[i - 1] == b'$' { i += 1; continue; }
            if i > 0 && bytes[i - 1] == b'\\' { i += 1; continue; }

            let open = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'$' && (i == 0 || bytes[i - 1] != b'\\') {
                    if i + 1 < bytes.len() && bytes[i + 1] == b'$' { i += 2; continue; }
                    let close = i + 1;
                    if !overlaps(&used, open, close) {
                        let inner = &content[open + 1..i];
                        let trimmed = inner.trim();
                        if !trimmed.is_empty() {
                            used.push((open, close));
                            equations.push(Equation {
                                content: trimmed.to_string(),
                                is_display: false,
                                start: open,
                                end: close,
                            });
                        }
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        i += 1;
    }

    equations.sort_by_key(|eq| eq.start);
    equations
}

/// Extract math equations from a Markdown document.
/// Skips math inside fenced code blocks or inline code.
pub fn extract_from_markdown(content: &str) -> Vec<Equation> {
    let code_ranges = find_code_ranges(content);
    let mut equations = extract_from_latex(content);
    equations.retain(|eq| {
        !code_ranges.iter().any(|&(s, e)| eq.start >= s && eq.end <= e)
    });
    equations
}

/// Find byte ranges of code blocks in Markdown.
fn find_code_ranges(content: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();

    // Fenced code blocks: find ``` or ~~~ and match until next one
    let lines: Vec<(usize, &str)> = content
        .split('\n')
        .scan(0usize, |pos, line| {
            let start = *pos;
            *pos += line.len() + 1; // +1 for newline
            Some((start, line))
        })
        .collect();

    let mut in_fence = false;
    let mut fence_start = 0;

    for &(offset, line) in &lines {
        let trimmed = line.trim();
        if !in_fence && (trimmed.starts_with("```") || trimmed.starts_with("~~~")) {
            in_fence = true;
            fence_start = offset;
        } else if in_fence && (trimmed.starts_with("```") || trimmed.starts_with("~~~")) {
            in_fence = false;
            ranges.push((fence_start, offset + line.len()));
        }
    }

    // Inline code: `...`
    let inline_re = Regex::new(r"`[^`\n]+`").unwrap();
    for m in inline_re.find_iter(content) {
        if !overlaps(&ranges, m.start(), m.end()) {
            ranges.push((m.start(), m.end()));
        }
    }

    ranges
}
