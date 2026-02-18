use std::path::PathBuf;
use std::process;

use anyhow::Result;

mod extract;
mod parser;
mod render;

use render::{Renderer, Theme};

struct Cli {
    input: PathBuf,
    output: PathBuf,
    theme: Theme,
    font_size: f32,
    scale: f32,
}

fn parse_args() -> Cli {
    let mut args = std::env::args().skip(1);
    let mut input = None;
    let mut output = PathBuf::from(".");
    let mut theme = Theme::Dark;
    let mut font_size = 24.0f32;
    let mut scale = 3.0f32;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-i" | "--input" => {
                input = args.next().map(PathBuf::from);
            }
            "-o" | "--output" => {
                if let Some(val) = args.next() {
                    output = PathBuf::from(val);
                }
            }
            "--theme" => {
                if let Some(val) = args.next() {
                    theme = match val.as_str() {
                        "light" => Theme::Light,
                        _ => Theme::Dark,
                    };
                }
            }
            "--font-size" => {
                if let Some(val) = args.next() {
                    font_size = val.parse().unwrap_or(24.0);
                }
            }
            "--scale" => {
                if let Some(val) = args.next() {
                    scale = val.parse().unwrap_or(3.0);
                }
            }
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            "-V" | "--version" => {
                println!("math2img {}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            other => {
                // Positional: treat as input if no -i given
                if input.is_none() {
                    input = Some(PathBuf::from(other));
                } else {
                    eprintln!("Unknown argument: {}", other);
                    print_help();
                    process::exit(1);
                }
            }
        }
    }

    let input = match input {
        Some(p) => p,
        None => {
            eprintln!("Error: input file is required");
            print_help();
            process::exit(1);
        }
    };

    Cli { input, output, theme, font_size, scale }
}

fn print_help() {
    eprintln!(
        "math2img - Extract math equations from LaTeX/Markdown and render as PNG

USAGE:
    math2img -i <INPUT> [OPTIONS]

OPTIONS:
    -i, --input <FILE>       Input file (LaTeX/TeX or Markdown)
    -o, --output <DIR>       Output directory for PNG images [default: .]
    --theme <dark|light>     Color theme [default: dark]
    --font-size <N>          Font size in points [default: 24]
    --scale <N>              Render scale factor [default: 3.0]
    -h, --help               Print help
    -V, --version            Print version"
    );
}

fn main() -> Result<()> {
    let cli = parse_args();

    let content = std::fs::read_to_string(&cli.input)
        .map_err(|e| anyhow::anyhow!("Failed to read {:?}: {}", cli.input, e))?;

    let is_markdown = matches!(
        cli.input.extension().and_then(|e| e.to_str()),
        Some("md" | "markdown" | "mdx")
    );

    let equations = if is_markdown {
        extract::extract_from_markdown(&content)
    } else {
        extract::extract_from_latex(&content)
    };

    if equations.is_empty() {
        eprintln!("No math equations found in {:?}", cli.input);
        return Ok(());
    }

    eprintln!("Found {} equation(s) in {:?}", equations.len(), cli.input);

    std::fs::create_dir_all(&cli.output)?;

    let renderer = Renderer::new();

    let mut success_count = 0;
    for (i, eq) in equations.iter().enumerate() {
        let output_path = cli.output.join(format!("equation_{:04}.png", i + 1));

        let ast = parser::parse(&eq.content);

        match renderer.render_equation(&ast, &cli.theme, cli.font_size, cli.scale, &output_path) {
            Ok(()) => {
                eprintln!(
                    "  [{}] {} -> {}",
                    if eq.is_display { "display" } else { "inline " },
                    truncate(&eq.content, 60),
                    output_path.display()
                );
                success_count += 1;
            }
            Err(e) => {
                eprintln!("  [ERROR] equation {}: {}", i + 1, e);
            }
        }
    }

    eprintln!(
        "Done. {}/{} equations rendered to {:?}",
        success_count,
        equations.len(),
        cli.output
    );
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ");
    if s.len() <= max_len {
        s
    } else {
        format!("{}...", &s[..max_len])
    }
}
