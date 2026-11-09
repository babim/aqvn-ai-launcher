//! Shell AST Lexer and Parser for SSH PTY Command Guard.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordPart {
    Literal(String),
    SingleQuoted(String),
    DoubleQuoted(Vec<WordPart>),
    AnsiCQuoted(String),
    Variable {
        name: String,
        default: Option<String>,
    },
    CommandSubst(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Word {
    pub parts: Vec<WordPart>,
}

impl Word {
    pub fn new(parts: Vec<WordPart>) -> Self {
        Self { parts }
    }

    pub fn from_literal(s: impl Into<String>) -> Self {
        Self {
            parts: vec![WordPart::Literal(s.into())],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Semicolon,
    Newline,
    And,
    Or,
    Pipe,
    Ampersand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedirectKind {
    Output, // >
    Append, // >>
    Input,  // <
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Redirect {
    pub kind: RedirectKind,
    pub target: Word,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub name: String,
    pub value: Word,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SimpleCommand {
    pub assignments: Vec<Assignment>,
    pub words: Vec<Word>,
    pub redirects: Vec<Redirect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Pipeline {
    pub commands: Vec<SimpleCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompoundItem {
    pub pipeline: Pipeline,
    pub op_after: Option<Operator>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ShellAst {
    pub items: Vec<CompoundItem>,
}

/// Token emitted by the low-level shell tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(Word),
    Operator(Operator),
    Redirect { kind: RedirectKind, target: Word },
    Assignment(Assignment),
}

/// Parses a shell command string into a structured Shell AST.
pub fn parse_shell_ast(input: &str) -> Result<ShellAst, String> {
    let tokens = tokenize(input)?;
    parse_tokens(tokens)
}

/// Tokenizer that decodes shell quoting, escapes, operators, assignments, and redirects.
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut at_command_start = true;

    while i < len {
        let c = chars[i];

        // Skip whitespace
        if c == ' ' || c == '\t' || c == '\r' {
            i += 1;
            continue;
        }

        // Comments: '#' outside quotes continues to end of line
        if c == '#' {
            while i < len && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Operators
        if c == '\n' {
            tokens.push(Token::Operator(Operator::Newline));
            at_command_start = true;
            i += 1;
            continue;
        }
        if c == ';' {
            tokens.push(Token::Operator(Operator::Semicolon));
            at_command_start = true;
            i += 1;
            continue;
        }
        if c == '&' {
            if i + 1 < len && chars[i + 1] == '&' {
                tokens.push(Token::Operator(Operator::And));
                i += 2;
            } else {
                tokens.push(Token::Operator(Operator::Ampersand));
                i += 1;
            }
            at_command_start = true;
            continue;
        }
        if c == '|' {
            if i + 1 < len && chars[i + 1] == '|' {
                tokens.push(Token::Operator(Operator::Or));
                at_command_start = true;
                i += 2;
            } else {
                tokens.push(Token::Operator(Operator::Pipe));
                at_command_start = true;
                i += 1;
            }
            continue;
        }

        // Grouping punctuation: '(', ')', '{', '}' acts as command separator
        if c == '(' || c == ')' || c == '{' || c == '}' {
            tokens.push(Token::Operator(Operator::Semicolon));
            at_command_start = true;
            i += 1;
            continue;
        }

        // Redirection
        if c == '>' {
            let (kind, next_i) = if i + 1 < len && chars[i + 1] == '>' {
                (RedirectKind::Append, i + 2)
            } else {
                (RedirectKind::Output, i + 1)
            };
            i = next_i;
            // Parse target word
            while i < len && (chars[i] == ' ' || chars[i] == '\t') {
                i += 1;
            }
            let (target_word, new_i) = parse_single_word(&chars, i)?;
            i = new_i;
            tokens.push(Token::Redirect {
                kind,
                target: target_word,
            });
            continue;
        }
        if c == '<' {
            i += 1;
            while i < len && (chars[i] == ' ' || chars[i] == '\t') {
                i += 1;
            }
            let (target_word, new_i) = parse_single_word(&chars, i)?;
            i = new_i;
            tokens.push(Token::Redirect {
                kind: RedirectKind::Input,
                target: target_word,
            });
            continue;
        }

        // Parse a word or assignment
        let (word, next_i) = parse_single_word(&chars, i)?;
        i = next_i;

        if at_command_start {
            if let Some(assignment) = try_extract_assignment(&word) {
                tokens.push(Token::Assignment(assignment));
                continue;
            }
        }

        at_command_start = false;
        tokens.push(Token::Word(word));
    }

    Ok(tokens)
}

fn try_extract_assignment(word: &Word) -> Option<Assignment> {
    if word.parts.is_empty() {
        return None;
    }
    // Check if the first part is a Literal with NAME=...
    if let WordPart::Literal(first_lit) = &word.parts[0] {
        if let Some(eq_pos) = first_lit.find('=') {
            let name = &first_lit[..eq_pos];
            if is_valid_identifier(name) {
                let rest_of_lit = &first_lit[eq_pos + 1..];
                let mut val_parts = Vec::new();
                if !rest_of_lit.is_empty() {
                    val_parts.push(WordPart::Literal(rest_of_lit.to_string()));
                }
                for part in word.parts.iter().skip(1) {
                    val_parts.push(part.clone());
                }
                return Some(Assignment {
                    name: name.to_string(),
                    value: Word::new(val_parts),
                });
            }
        }
    }
    None
}

fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn parse_single_word(chars: &[char], mut i: usize) -> Result<(Word, usize), String> {
    let mut parts = Vec::new();
    let len = chars.len();

    while i < len {
        let c = chars[i];
        if c == ' '
            || c == '\t'
            || c == '\r'
            || c == '\n'
            || c == ';'
            || c == '&'
            || c == '|'
            || c == '>'
            || c == '<'
            || c == '('
            || c == ')'
            || c == '{'
            || c == '}'
        {
            break;
        }

        // ANSI-C quoting: $'...'
        if c == '$' && i + 1 < len && chars[i + 1] == '\'' {
            let (content, next_i) = parse_ansi_c_quote(chars, i + 2)?;
            parts.push(WordPart::AnsiCQuoted(content));
            i = next_i;
            continue;
        }

        // Single quote: '...'
        if c == '\'' {
            let (content, next_i) = parse_single_quote(chars, i + 1)?;
            parts.push(WordPart::SingleQuoted(content));
            i = next_i;
            continue;
        }

        // Double quote: "..."
        if c == '"' {
            let (subparts, next_i) = parse_double_quote(chars, i + 1)?;
            parts.push(WordPart::DoubleQuoted(subparts));
            i = next_i;
            continue;
        }

        // Backslash escape outside quotes: \c
        if c == '\\' {
            if i + 1 < len {
                let escaped_c = chars[i + 1];
                parts.push(WordPart::Literal(escaped_c.to_string()));
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }

        // Variable or command substitution: $...
        if c == '$' {
            if i + 1 < len && chars[i + 1] == '(' {
                // Command substitution: $(...)
                let (sub_cmd, next_i) = parse_balanced_parens(chars, i + 2)?;
                parts.push(WordPart::CommandSubst(sub_cmd));
                i = next_i;
                continue;
            } else if i + 1 < len && chars[i + 1] == '{' {
                // Parameter expansion: ${VAR} or ${VAR:-default}
                let (var_part, next_i) = parse_braced_var(chars, i + 2)?;
                parts.push(var_part);
                i = next_i;
                continue;
            } else if i + 1 < len && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '_') {
                // Simple variable: $VAR
                let mut var_name = String::new();
                let mut vi = i + 1;
                while vi < len && (chars[vi].is_ascii_alphanumeric() || chars[vi] == '_') {
                    var_name.push(chars[vi]);
                    vi += 1;
                }
                parts.push(WordPart::Variable {
                    name: var_name,
                    default: None,
                });
                i = vi;
                continue;
            } else {
                parts.push(WordPart::Literal("$".to_string()));
                i += 1;
                continue;
            }
        }

        // Backtick command substitution: `...`
        if c == '`' {
            let (sub_cmd, next_i) = parse_backticks(chars, i + 1)?;
            parts.push(WordPart::CommandSubst(sub_cmd));
            i = next_i;
            continue;
        }

        // Ordinary character - accumulate contiguous unescaped characters
        let mut literal = String::new();
        while i < len {
            let cur = chars[i];
            if cur == ' '
                || cur == '\t'
                || cur == '\r'
                || cur == '\n'
                || cur == ';'
                || cur == '&'
                || cur == '|'
                || cur == '>'
                || cur == '<'
                || cur == '('
                || cur == ')'
                || cur == '{'
                || cur == '}'
                || cur == '\''
                || cur == '"'
                || cur == '\\'
                || cur == '$'
                || cur == '`'
            {
                break;
            }
            literal.push(cur);
            i += 1;
        }
        if !literal.is_empty() {
            parts.push(WordPart::Literal(literal));
        }
    }

    Ok((Word::new(parts), i))
}

fn parse_single_quote(chars: &[char], mut i: usize) -> Result<(String, usize), String> {
    let mut content = String::new();
    let len = chars.len();
    while i < len && chars[i] != '\'' {
        content.push(chars[i]);
        i += 1;
    }
    if i >= len {
        // Unclosed single quote - graceful tolerance
        return Ok((content, i));
    }
    Ok((content, i + 1))
}

fn parse_ansi_c_quote(chars: &[char], mut i: usize) -> Result<(String, usize), String> {
    let mut content = String::new();
    let len = chars.len();
    while i < len && chars[i] != '\'' {
        if chars[i] == '\\' && i + 1 < len {
            match chars[i + 1] {
                'x' => {
                    // \xHH
                    let mut hex = String::new();
                    let mut hi = i + 2;
                    while hi < len && hi < i + 4 && chars[hi].is_ascii_hexdigit() {
                        hex.push(chars[hi]);
                        hi += 1;
                    }
                    if let Ok(b) = u8::from_str_radix(&hex, 16) {
                        content.push(b as char);
                        i = hi;
                    } else {
                        content.push('\\');
                        content.push('x');
                        i += 2;
                    }
                }
                'n' => {
                    content.push('\n');
                    i += 2;
                }
                'r' => {
                    content.push('\r');
                    i += 2;
                }
                't' => {
                    content.push('\t');
                    i += 2;
                }
                '\\' => {
                    content.push('\\');
                    i += 2;
                }
                '\'' => {
                    content.push('\'');
                    i += 2;
                }
                '"' => {
                    content.push('"');
                    i += 2;
                }
                other => {
                    content.push(other);
                    i += 2;
                }
            }
        } else {
            content.push(chars[i]);
            i += 1;
        }
    }
    if i >= len {
        return Ok((content, i));
    }
    Ok((content, i + 1))
}

fn parse_double_quote(chars: &[char], mut i: usize) -> Result<(Vec<WordPart>, usize), String> {
    let mut parts = Vec::new();
    let len = chars.len();

    while i < len && chars[i] != '"' {
        let c = chars[i];
        if c == '\\' && i + 1 < len {
            let next_c = chars[i + 1];
            if next_c == '"' || next_c == '\\' || next_c == '$' || next_c == '`' {
                parts.push(WordPart::Literal(next_c.to_string()));
            } else {
                parts.push(WordPart::Literal(format!("\\{next_c}")));
            }
            i += 2;
            continue;
        }

        if c == '$' {
            if i + 1 < len && chars[i + 1] == '(' {
                let (sub_cmd, next_i) = parse_balanced_parens(chars, i + 2)?;
                parts.push(WordPart::CommandSubst(sub_cmd));
                i = next_i;
                continue;
            } else if i + 1 < len && chars[i + 1] == '{' {
                let (var_part, next_i) = parse_braced_var(chars, i + 2)?;
                parts.push(var_part);
                i = next_i;
                continue;
            } else if i + 1 < len && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '_') {
                let mut var_name = String::new();
                let mut vi = i + 1;
                while vi < len && (chars[vi].is_ascii_alphanumeric() || chars[vi] == '_') {
                    var_name.push(chars[vi]);
                    vi += 1;
                }
                parts.push(WordPart::Variable {
                    name: var_name,
                    default: None,
                });
                i = vi;
                continue;
            }
        }

        if c == '`' {
            let (sub_cmd, next_i) = parse_backticks(chars, i + 1)?;
            parts.push(WordPart::CommandSubst(sub_cmd));
            i = next_i;
            continue;
        }

        let mut lit = String::new();
        while i < len && chars[i] != '"' && chars[i] != '\\' && chars[i] != '$' && chars[i] != '`' {
            lit.push(chars[i]);
            i += 1;
        }
        if !lit.is_empty() {
            parts.push(WordPart::Literal(lit));
        }
    }

    if i < len && chars[i] == '"' {
        i += 1;
    }
    Ok((parts, i))
}

fn parse_balanced_parens(chars: &[char], mut i: usize) -> Result<(String, usize), String> {
    let mut depth = 1;
    let mut content = String::new();
    let len = chars.len();

    while i < len {
        let c = chars[i];
        if c == '(' {
            depth += 1;
            content.push(c);
            i += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 {
                return Ok((content, i + 1));
            }
            content.push(c);
            i += 1;
        } else if c == '\'' {
            // Quote inside subshell
            content.push(c);
            i += 1;
            while i < len && chars[i] != '\'' {
                content.push(chars[i]);
                i += 1;
            }
            if i < len {
                content.push(chars[i]);
                i += 1;
            }
        } else if c == '"' {
            content.push(c);
            i += 1;
            while i < len && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < len {
                    content.push(chars[i]);
                    content.push(chars[i + 1]);
                    i += 2;
                } else {
                    content.push(chars[i]);
                    i += 1;
                }
            }
            if i < len {
                content.push(chars[i]);
                i += 1;
            }
        } else {
            content.push(c);
            i += 1;
        }
    }

    Ok((content, i))
}

fn parse_backticks(chars: &[char], mut i: usize) -> Result<(String, usize), String> {
    let mut content = String::new();
    let len = chars.len();
    while i < len && chars[i] != '`' {
        if chars[i] == '\\' && i + 1 < len {
            content.push(chars[i + 1]);
            i += 2;
        } else {
            content.push(chars[i]);
            i += 1;
        }
    }
    if i < len && chars[i] == '`' {
        i += 1;
    }
    Ok((content, i))
}

fn parse_braced_var(chars: &[char], mut i: usize) -> Result<(WordPart, usize), String> {
    let mut expr = String::new();
    let len = chars.len();
    while i < len && chars[i] != '}' {
        expr.push(chars[i]);
        i += 1;
    }
    if i < len && chars[i] == '}' {
        i += 1;
    }

    if let Some((name, default)) = expr.split_once(":-") {
        Ok((
            WordPart::Variable {
                name: name.trim().to_string(),
                default: Some(default.to_string()),
            },
            i,
        ))
    } else if let Some((name, default)) = expr.split_once('-') {
        Ok((
            WordPart::Variable {
                name: name.trim().to_string(),
                default: Some(default.to_string()),
            },
            i,
        ))
    } else {
        Ok((
            WordPart::Variable {
                name: expr.trim().to_string(),
                default: None,
            },
            i,
        ))
    }
}

/// Parses the linear stream of tokens into a ShellAst.
fn parse_tokens(tokens: Vec<Token>) -> Result<ShellAst, String> {
    let mut items = Vec::new();
    let mut current_pipeline = Pipeline::default();
    let mut current_cmd = SimpleCommand::default();

    for token in tokens {
        match token {
            Token::Assignment(assign) => {
                current_cmd.assignments.push(assign);
            }
            Token::Word(word) => {
                current_cmd.words.push(word);
            }
            Token::Redirect { kind, target } => {
                current_cmd.redirects.push(Redirect { kind, target });
            }
            Token::Operator(Operator::Pipe) => {
                if !current_cmd.words.is_empty()
                    || !current_cmd.assignments.is_empty()
                    || !current_cmd.redirects.is_empty()
                {
                    current_pipeline.commands.push(current_cmd);
                    current_cmd = SimpleCommand::default();
                }
            }
            Token::Operator(op) => {
                if !current_cmd.words.is_empty()
                    || !current_cmd.assignments.is_empty()
                    || !current_cmd.redirects.is_empty()
                {
                    current_pipeline.commands.push(current_cmd);
                    current_cmd = SimpleCommand::default();
                }
                if !current_pipeline.commands.is_empty() {
                    items.push(CompoundItem {
                        pipeline: current_pipeline,
                        op_after: Some(op),
                    });
                    current_pipeline = Pipeline::default();
                }
            }
        }
    }

    if !current_cmd.words.is_empty()
        || !current_cmd.assignments.is_empty()
        || !current_cmd.redirects.is_empty()
    {
        current_pipeline.commands.push(current_cmd);
    }
    if !current_pipeline.commands.is_empty() {
        items.push(CompoundItem {
            pipeline: current_pipeline,
            op_after: None,
        });
    }

    Ok(ShellAst { items })
}
