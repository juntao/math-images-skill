/// LaTeX math parser → AST

#[derive(Debug, Clone)]
pub enum MathNode {
    Symbol(char),
    Row(Vec<MathNode>),
    Frac(Box<MathNode>, Box<MathNode>),
    Sup(Box<MathNode>, Box<MathNode>),
    Sub(Box<MathNode>, Box<MathNode>),
    SubSup(Box<MathNode>, Box<MathNode>, Box<MathNode>),
    Sqrt(Box<MathNode>),
    Matrix {
        rows: Vec<Vec<MathNode>>,
        left_delim: Option<char>,
        right_delim: Option<char>,
    },
    Text(String),
    Space(f32),
    Overline(Box<MathNode>),
    Delimited {
        left: char,
        right: char,
        content: Box<MathNode>,
    },
    Accent(char, Box<MathNode>),
    Cases(Vec<Vec<MathNode>>),
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(input: &str) -> Self {
        Parser {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.advance();
        }
    }

    fn eat(&mut self, ch: char) -> bool {
        self.skip_ws();
        if self.peek() == Some(ch) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn read_until(&mut self, stop: char) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch == stop {
                break;
            }
            s.push(ch);
            self.advance();
        }
        s
    }

    fn read_cmd(&mut self) -> String {
        let mut cmd = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphabetic() {
                cmd.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        if cmd.is_empty() {
            if let Some(ch) = self.advance() {
                cmd.push(ch);
            }
        }
        cmd
    }

    fn read_env_name(&mut self) -> String {
        self.eat('{');
        let name = self.read_until('}');
        self.eat('}');
        name
    }

    fn read_group(&mut self) -> MathNode {
        self.skip_ws();
        if self.eat('{') {
            let node = self.parse_expr_until(|c| c == '}');
            self.eat('}');
            node
        } else {
            self.parse_single_atom()
                .unwrap_or(MathNode::Row(vec![]))
        }
    }

    /// Parse a single atom without handling scripts.
    fn parse_single_atom(&mut self) -> Option<MathNode> {
        self.skip_ws();
        let ch = self.peek()?;
        match ch {
            '\\' => {
                self.advance();
                let cmd = self.read_cmd();
                self.dispatch_cmd(&cmd)
            }
            '{' => Some(self.read_group()),
            '}' | '&' | '^' | '_' => None,
            c if is_math_char(c) => {
                self.advance();
                Some(MathNode::Symbol(c))
            }
            _ => {
                self.advance();
                Some(MathNode::Symbol(ch))
            }
        }
    }

    /// Parse expression: sequence of atoms with scripts attached.
    fn parse_expr_until(&mut self, stop: impl Fn(char) -> bool) -> MathNode {
        let mut nodes = Vec::new();

        loop {
            self.skip_ws();
            match self.peek() {
                None => break,
                Some(c) if stop(c) => break,
                Some('\\') => {
                    // Peek for \end or \right
                    let saved = self.pos;
                    self.advance();
                    let cmd = self.read_cmd();
                    if cmd == "end" || cmd == "right" || cmd == "\\" {
                        self.pos = saved;
                        break;
                    }
                    self.pos = saved;
                    match self.parse_single_atom() {
                        Some(n) => nodes.push(self.maybe_scripts(n)),
                        None => break,
                    }
                }
                Some('^') | Some('_') => {
                    // Script without explicit base — use empty row
                    let base = MathNode::Row(vec![]);
                    nodes.push(self.maybe_scripts(base));
                }
                _ => match self.parse_single_atom() {
                    Some(n) => nodes.push(self.maybe_scripts(n)),
                    None => break,
                },
            }
        }

        if nodes.len() == 1 {
            nodes.pop().unwrap()
        } else {
            MathNode::Row(nodes)
        }
    }

    /// After parsing an atom, check for ^ and _ to attach scripts.
    fn maybe_scripts(&mut self, base: MathNode) -> MathNode {
        let mut sup = None;
        let mut sub = None;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('^') if sup.is_none() => {
                    self.advance();
                    sup = Some(Box::new(self.read_group()));
                }
                Some('_') if sub.is_none() => {
                    self.advance();
                    sub = Some(Box::new(self.read_group()));
                }
                _ => break,
            }
        }
        match (sub, sup) {
            (Some(sb), Some(sp)) => MathNode::SubSup(Box::new(base), sb, sp),
            (None, Some(sp)) => MathNode::Sup(Box::new(base), sp),
            (Some(sb), None) => MathNode::Sub(Box::new(base), sb),
            (None, None) => base,
        }
    }

    fn dispatch_cmd(&mut self, cmd: &str) -> Option<MathNode> {
        match cmd {
            // Fractions
            "frac" | "dfrac" | "tfrac" => {
                let num = self.read_group();
                let den = self.read_group();
                Some(MathNode::Frac(Box::new(num), Box::new(den)))
            }
            "sqrt" => {
                let content = self.read_group();
                Some(MathNode::Sqrt(Box::new(content)))
            }
            "overline" | "bar" => {
                let c = self.read_group();
                Some(MathNode::Overline(Box::new(c)))
            }
            "hat" => { let c = self.read_group(); Some(MathNode::Accent('\u{0302}', Box::new(c))) }
            "tilde" => { let c = self.read_group(); Some(MathNode::Accent('~', Box::new(c))) }
            "vec" => { let c = self.read_group(); Some(MathNode::Accent('\u{2192}', Box::new(c))) }
            "dot" => { let c = self.read_group(); Some(MathNode::Accent('\u{02D9}', Box::new(c))) }
            "ddot" => { let c = self.read_group(); Some(MathNode::Accent('\u{00A8}', Box::new(c))) }

            // Text
            "text" | "textrm" | "mathrm" | "operatorname" => {
                self.eat('{');
                let t = self.read_until('}');
                self.eat('}');
                Some(MathNode::Text(t))
            }
            "mathbf" | "textbf" | "boldsymbol" | "bm"
            | "mathit" | "textit"
            | "mathcal" | "mathbb" | "mathfrak" | "mathsf" | "mathtt" => {
                Some(self.read_group())
            }

            // Greek lowercase
            "alpha" => sym('\u{03B1}'), "beta" => sym('\u{03B2}'),
            "gamma" => sym('\u{03B3}'), "delta" => sym('\u{03B4}'),
            "epsilon" | "varepsilon" => sym('\u{03B5}'),
            "zeta" => sym('\u{03B6}'), "eta" => sym('\u{03B7}'),
            "theta" | "vartheta" => sym('\u{03B8}'),
            "iota" => sym('\u{03B9}'), "kappa" => sym('\u{03BA}'),
            "lambda" => sym('\u{03BB}'), "mu" => sym('\u{03BC}'),
            "nu" => sym('\u{03BD}'), "xi" => sym('\u{03BE}'),
            "pi" | "varpi" => sym('\u{03C0}'),
            "rho" | "varrho" => sym('\u{03C1}'),
            "sigma" | "varsigma" => sym('\u{03C3}'),
            "tau" => sym('\u{03C4}'), "upsilon" => sym('\u{03C5}'),
            "phi" | "varphi" => sym('\u{03C6}'),
            "chi" => sym('\u{03C7}'), "psi" => sym('\u{03C8}'),
            "omega" => sym('\u{03C9}'),

            // Greek uppercase
            "Gamma" => sym('\u{0393}'), "Delta" => sym('\u{0394}'),
            "Theta" => sym('\u{0398}'), "Lambda" => sym('\u{039B}'),
            "Xi" => sym('\u{039E}'), "Pi" => sym('\u{03A0}'),
            "Sigma" => sym('\u{03A3}'), "Upsilon" => sym('\u{03A5}'),
            "Phi" => sym('\u{03A6}'), "Psi" => sym('\u{03A8}'),
            "Omega" => sym('\u{03A9}'),

            // Big operators
            "sum" => sym('\u{2211}'), "prod" => sym('\u{220F}'),
            "int" => sym('\u{222B}'), "iint" => sym('\u{222C}'),
            "iiint" => sym('\u{222D}'), "oint" => sym('\u{222E}'),
            "bigcup" => sym('\u{22C3}'), "bigcap" => sym('\u{22C2}'),
            "bigoplus" => sym('\u{2A01}'), "bigotimes" => sym('\u{2A02}'),
            "coprod" => sym('\u{2210}'),

            // Binary operators
            "times" => sym('\u{00D7}'), "div" => sym('\u{00F7}'),
            "cdot" => sym('\u{22C5}'), "pm" => sym('\u{00B1}'),
            "mp" => sym('\u{2213}'), "circ" => sym('\u{2218}'),
            "ast" => sym('\u{2217}'), "star" => sym('\u{22C6}'),
            "oplus" => sym('\u{2295}'), "otimes" => sym('\u{2297}'),

            // Relations
            "leq" | "le" => sym('\u{2264}'), "geq" | "ge" => sym('\u{2265}'),
            "neq" | "ne" => sym('\u{2260}'), "approx" => sym('\u{2248}'),
            "equiv" => sym('\u{2261}'), "sim" => sym('\u{223C}'),
            "simeq" => sym('\u{2243}'), "cong" => sym('\u{2245}'),
            "propto" => sym('\u{221D}'),
            "subset" => sym('\u{2282}'), "supset" => sym('\u{2283}'),
            "subseteq" => sym('\u{2286}'), "supseteq" => sym('\u{2287}'),
            "in" => sym('\u{2208}'), "notin" => sym('\u{2209}'),
            "ni" => sym('\u{220B}'),
            "cup" => sym('\u{222A}'), "cap" => sym('\u{2229}'),
            "vee" | "lor" => sym('\u{2228}'), "wedge" | "land" => sym('\u{2227}'),
            "perp" => sym('\u{22A5}'), "parallel" => sym('\u{2225}'),
            "mid" => sym('|'), "ll" => sym('\u{226A}'), "gg" => sym('\u{226B}'),
            "prec" => sym('\u{227A}'), "succ" => sym('\u{227B}'),

            // Arrows
            "to" | "rightarrow" => sym('\u{2192}'),
            "leftarrow" | "gets" => sym('\u{2190}'),
            "leftrightarrow" => sym('\u{2194}'),
            "Rightarrow" => sym('\u{21D2}'),
            "Leftarrow" => sym('\u{21D0}'),
            "Leftrightarrow" | "iff" => sym('\u{21D4}'),
            "uparrow" => sym('\u{2191}'), "downarrow" => sym('\u{2193}'),
            "mapsto" => sym('\u{21A6}'),
            "hookrightarrow" => sym('\u{21AA}'),
            "longrightarrow" => sym('\u{27F6}'),
            "Longrightarrow" => sym('\u{27F9}'),

            // Misc
            "infty" => sym('\u{221E}'), "partial" => sym('\u{2202}'),
            "nabla" => sym('\u{2207}'), "forall" => sym('\u{2200}'),
            "exists" => sym('\u{2203}'), "nexists" => sym('\u{2204}'),
            "emptyset" | "varnothing" => sym('\u{2205}'),
            "neg" | "lnot" => sym('\u{00AC}'),
            "angle" => sym('\u{2220}'), "triangle" => sym('\u{25B3}'),
            "prime" => sym('\u{2032}'), "hbar" => sym('\u{210F}'),
            "ell" => sym('\u{2113}'), "aleph" => sym('\u{2135}'),
            "Re" => sym('\u{211C}'), "Im" => sym('\u{2111}'),

            // Dots
            "ldots" | "dots" => sym('\u{2026}'),
            "cdots" => sym('\u{22EF}'),
            "vdots" => sym('\u{22EE}'),
            "ddots" => sym('\u{22F1}'),

            // Delimiters
            "langle" => sym('\u{27E8}'), "rangle" => sym('\u{27E9}'),
            "lceil" => sym('\u{2308}'), "rceil" => sym('\u{2309}'),
            "lfloor" => sym('\u{230A}'), "rfloor" => sym('\u{230B}'),
            "lbrace" | "{" => sym('{'), "rbrace" | "}" => sym('}'),
            "lvert" | "rvert" => sym('|'),
            "lVert" | "rVert" | "|" => sym('\u{2016}'),

            // Spacing
            "," => Some(MathNode::Space(0.17)),
            ":" | ">" => Some(MathNode::Space(0.22)),
            ";" => Some(MathNode::Space(0.28)),
            "!" => Some(MathNode::Space(-0.17)),
            "quad" => Some(MathNode::Space(1.0)),
            "qquad" => Some(MathNode::Space(2.0)),
            " " => Some(MathNode::Space(0.25)),

            // Function names
            "sin" | "cos" | "tan" | "cot" | "sec" | "csc"
            | "arcsin" | "arccos" | "arctan"
            | "sinh" | "cosh" | "tanh" | "coth"
            | "log" | "ln" | "exp"
            | "lim" | "limsup" | "liminf"
            | "max" | "min" | "sup" | "inf"
            | "det" | "gcd" | "lcm" | "dim" | "ker" | "deg"
            | "arg" | "hom" | "Pr" | "mod" => {
                Some(MathNode::Text(cmd.to_string()))
            }

            // \left ... \right
            "left" => {
                let ld = self.read_delim_char();
                let content = self.parse_expr_until(|_| false);
                // We exited because we hit \right
                self.advance(); // skip '\'
                self.read_cmd(); // consume "right"
                let rd = self.read_delim_char();
                Some(MathNode::Delimited {
                    left: ld,
                    right: rd,
                    content: Box::new(content),
                })
            }
            "right" => None,

            // Environments
            "begin" => {
                let env = self.read_env_name();
                self.parse_env(&env)
            }
            "end" => {
                self.read_env_name();
                None
            }

            // Line break (in aligned envs)
            "\\" => None,

            "not" => {
                self.parse_single_atom()
            }

            _ => Some(MathNode::Text(format!("\\{}", cmd))),
        }
    }

    fn read_delim_char(&mut self) -> char {
        self.skip_ws();
        if let Some(ch) = self.peek() {
            if ch == '\\' {
                self.advance();
                let cmd = self.read_cmd();
                match cmd.as_str() {
                    "{" | "lbrace" => '{',
                    "}" | "rbrace" => '}',
                    "|" | "lVert" | "rVert" => '\u{2016}',
                    "langle" => '\u{27E8}',
                    "rangle" => '\u{27E9}',
                    "lceil" => '\u{2308}',
                    "rceil" => '\u{2309}',
                    "lfloor" => '\u{230A}',
                    "rfloor" => '\u{230B}',
                    "lvert" | "rvert" => '|',
                    _ => '.',
                }
            } else {
                self.advance();
                if ch == '.' { '\0' } else { ch }
            }
        } else {
            '\0'
        }
    }

    fn parse_env(&mut self, env: &str) -> Option<MathNode> {
        match env {
            "pmatrix" => self.parse_matrix(Some('('), Some(')')),
            "bmatrix" => self.parse_matrix(Some('['), Some(']')),
            "Bmatrix" => self.parse_matrix(Some('{'), Some('}')),
            "vmatrix" => self.parse_matrix(Some('|'), Some('|')),
            "Vmatrix" => self.parse_matrix(Some('\u{2016}'), Some('\u{2016}')),
            "matrix" | "smallmatrix" => self.parse_matrix(None, None),
            "cases" => {
                let rows = self.parse_tabular();
                Some(MathNode::Cases(rows))
            }
            "array" => {
                self.skip_ws();
                if self.peek() == Some('{') {
                    self.eat('{');
                    self.read_until('}');
                    self.eat('}');
                }
                self.parse_matrix(None, None)
            }
            _ => {
                // aligned, gather, equation, etc. — parse as rows
                let rows = self.parse_tabular();
                if rows.len() == 1 && rows[0].len() == 1 {
                    Some(rows.into_iter().next().unwrap().into_iter().next().unwrap())
                } else {
                    Some(MathNode::Matrix {
                        rows,
                        left_delim: None,
                        right_delim: None,
                    })
                }
            }
        }
    }

    fn parse_matrix(&mut self, left: Option<char>, right: Option<char>) -> Option<MathNode> {
        let rows = self.parse_tabular();
        Some(MathNode::Matrix {
            rows,
            left_delim: left,
            right_delim: right,
        })
    }

    /// Parse tabular content (& separates cells, \\ separates rows) until \end{...}
    fn parse_tabular(&mut self) -> Vec<Vec<MathNode>> {
        let mut rows: Vec<Vec<MathNode>> = Vec::new();
        let mut row: Vec<MathNode> = Vec::new();

        loop {
            let cell = self.parse_expr_until(|c| c == '&');

            self.skip_ws();
            match self.peek() {
                Some('&') => {
                    self.advance();
                    row.push(cell);
                }
                Some('\\') => {
                    let saved = self.pos;
                    self.advance();
                    let cmd = self.read_cmd();
                    if cmd == "end" {
                        self.read_env_name();
                        row.push(cell);
                        break;
                    } else if cmd == "\\" {
                        row.push(cell);
                        rows.push(row);
                        row = Vec::new();
                        // Skip optional [...]
                        self.skip_ws();
                        if self.peek() == Some('[') {
                            self.advance();
                            self.read_until(']');
                            self.eat(']');
                        }
                    } else if cmd == "hline" || cmd == "cline" {
                        if cmd == "cline" {
                            self.eat('{');
                            self.read_until('}');
                            self.eat('}');
                        }
                        // Don't push cell, continue
                    } else {
                        self.pos = saved;
                        row.push(cell);
                        break;
                    }
                }
                _ => {
                    row.push(cell);
                    break;
                }
            }
        }
        if !row.is_empty() {
            rows.push(row);
        }
        rows
    }
}

fn sym(c: char) -> Option<MathNode> {
    Some(MathNode::Symbol(c))
}

fn is_math_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            '+' | '-' | '*' | '/' | '=' | '<' | '>' | '(' | ')' | '[' | ']'
            | '|' | '!' | '?' | ',' | ';' | ':' | '.' | '\''
        )
}

/// Parse a LaTeX math expression into an AST.
pub fn parse(input: &str) -> MathNode {
    let input = strip_env_wrapper(input);
    let mut p = Parser::new(&input);
    p.parse_expr_until(|_| false)
}

fn strip_env_wrapper(input: &str) -> String {
    let input = input.trim();
    let re = regex_lite::Regex::new(
        r"(?s)^\\begin\{(equation\*?|displaymath|math)\}(.*?)\\end\{(equation\*?|displaymath|math)\}$"
    )
    .unwrap();
    if let Some(cap) = re.captures(input) {
        if cap.get(1).map(|m| m.as_str()) == cap.get(3).map(|m| m.as_str()) {
            return cap[2].trim().to_string();
        }
    }
    input.to_string()
}
