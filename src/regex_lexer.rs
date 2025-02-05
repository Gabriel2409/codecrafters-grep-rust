#[derive(Debug, PartialEq, Clone)]
pub enum RegexToken {
    /// Literal char in pattern
    Literal(char),
    /// \d in pattern
    Digit,
    /// \w in pattern
    AlphaNum,
    /// Quantifier, ?, * or {x} or {x,y} or {x,}
    Quantifier {
        min: usize,
        max: Option<usize>, // None for infinity
    },
    /// (
    LParen,
    /// )
    RParen,
    /// [
    LBracket,
    /// ]
    RBracket,
    /// |
    Pipe,
    /// End of input
    Eof,
    /// \15
    BackRef(usize),
    /// ^
    StartAnchor,
    /// $
    EndAnchor,
    /// .
    Wildcard,
}

/// Overengineered struct to transform the pattern into a set of tokens
#[derive(Debug)]
pub struct RegexLexer {
    /// pattern string as a vec of chars
    /// It is not optimal as we have to collect all the chars first but the
    /// input string is often quite short
    /// This method allows to include non ASCII chars.
    /// If we only use ascii chars, we can instead make ch a u8
    chars: Vec<char>,
    /// current position in input (points to current char)
    position: usize,
    /// current reading position in input (after current char)
    read_position: usize,
    /// current char under examination (None for EOF)
    ch: Option<char>,
}

impl RegexLexer {
    pub fn new(input: &str) -> Self {
        let chars = input.chars().collect::<Vec<_>>();

        let mut regex_lexer = Self {
            chars,
            position: 0,
            read_position: 0,
            ch: None,
        };
        regex_lexer.read_char();
        regex_lexer
    }

    pub fn read_char(&mut self) {
        if self.read_position >= self.chars.len() {
            self.ch = None
        } else {
            self.ch = Some(self.chars[self.read_position].clone());
        }
        self.position = self.read_position;
        self.read_position += 1;
    }

    pub fn peek_char(&self) -> Option<char> {
        if self.read_position >= self.chars.len() {
            None
        } else {
            Some(self.chars[self.read_position].clone())
        }
    }

    pub fn read_number(&mut self) -> anyhow::Result<usize> {
        let mut s = String::new();

        while let Some(c) = self.ch {
            if c.is_ascii_digit() {
                s.push(c);
                match self.peek_char() {
                    Some(c) if c.is_ascii_digit() => {
                        self.read_char();
                    }
                    _ => {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        Ok(s.parse::<usize>()?)
    }

    pub fn read_brace_quantifier(&mut self) -> anyhow::Result<RegexToken> {
        self.read_char();
        let min = self.read_number()?;
        self.read_char();
        let max;
        match self.ch {
            Some('}') => {
                max = Some(min);
            }
            Some(',') => {
                if let Some('}') = self.peek_char() {
                    max = None;
                    self.read_char();
                } else {
                    self.read_char();
                    max = Some(self.read_number()?);
                    self.read_char();

                    if self.ch.unwrap() != '}' {
                        println!("Problem parsing braces");
                        std::process::exit(1);
                    }
                }
            }
            _ => {
                println!("Problem parsing braces");
                std::process::exit(1);
            }
        }

        Ok(RegexToken::Quantifier { min, max })
    }

    pub fn next_token(&mut self) -> anyhow::Result<RegexToken> {
        let tok = match self.ch {
            None => RegexToken::Eof,
            Some(c) => match c {
                '|' => RegexToken::Pipe,
                '(' => RegexToken::LParen,
                ')' => RegexToken::RParen,
                '[' => RegexToken::LBracket,
                ']' => RegexToken::RBracket,
                '^' => RegexToken::StartAnchor,
                '$' => RegexToken::EndAnchor,
                '.' => RegexToken::Wildcard,
                '*' => RegexToken::Quantifier { min: 0, max: None },
                '+' => RegexToken::Quantifier { min: 1, max: None },
                '?' => RegexToken::Quantifier {
                    min: 0,
                    max: Some(1),
                },
                '\\' => match self.peek_char() {
                    Some('w') => {
                        let tok = RegexToken::AlphaNum;
                        self.read_char();
                        tok
                    }
                    Some('d') => {
                        let tok = RegexToken::Digit;
                        self.read_char();
                        tok
                    }
                    Some(x) if x.is_ascii_digit() => {
                        self.read_char();
                        let num = self.read_number()?;
                        RegexToken::BackRef(num)
                    }
                    // Not exactly correct but let's consider we need to escape punctuation
                    Some(x) if x.is_ascii_punctuation() => {
                        let tok = RegexToken::Literal(x);
                        self.read_char();
                        tok
                    }
                    _ => {
                        println!("Error parsing expression");
                        std::process::exit(1);
                    }
                },
                '{' => self.read_brace_quantifier()?,
                x => RegexToken::Literal(x),
            },
        };
        self.read_char();
        Ok(tok)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("ab?", vec![RegexToken::Literal('a'), RegexToken::Literal('b'), RegexToken::Quantifier { min: 0, max: Some(1) }])]
    #[case("(a|bc){1,3}\\12", vec![RegexToken::LParen,RegexToken::Literal('a'), RegexToken::Pipe, RegexToken::Literal('b'), RegexToken::Literal('c'), RegexToken::RParen, RegexToken::Quantifier { min: 1, max: Some(3) }, RegexToken::BackRef(12)])]
    #[case("a*\\d+", vec![RegexToken::Literal('a'), RegexToken::Quantifier { min: 0, max: None }, RegexToken::Digit, RegexToken::Quantifier{min:1, max:None}])]
    #[case("a*\\wb", vec![RegexToken::Literal('a'), RegexToken::Quantifier { min: 0, max: None }, RegexToken::AlphaNum, RegexToken::Literal('b')])]
    #[case("a{1}b", vec![RegexToken::Literal('a'), RegexToken::Quantifier { min: 1, max: Some(1) }, RegexToken::Literal('b')])]
    #[case("a{1,}b", vec![RegexToken::Literal('a'), RegexToken::Quantifier { min: 1, max: None }, RegexToken::Literal('b')])]
    #[case("a[bwz]b", vec![RegexToken::Literal('a'), RegexToken::LBracket , RegexToken::Literal('b'), RegexToken::Literal('w'), RegexToken::Literal('z'), RegexToken::RBracket, RegexToken::Literal('b')])]
    #[case("^a.b$", vec![RegexToken::StartAnchor,RegexToken::Literal('a'), RegexToken::Wildcard, RegexToken::Literal('b'), RegexToken::EndAnchor])]
    #[case(r#"a\{"#, vec![RegexToken::Literal('a'), RegexToken::Literal('{')])]
    fn test_lexer(#[case] pat: &str, #[case] expected: Vec<RegexToken>) -> anyhow::Result<()> {
        let mut lexer = RegexLexer::new(pat);

        let mut tokens = Vec::new();
        loop {
            let token = lexer.next_token()?;
            if let RegexToken::Eof = token {
                break;
            }
            tokens.push(token);
        }

        assert_eq!(tokens, expected);
        Ok(())
    }
}
