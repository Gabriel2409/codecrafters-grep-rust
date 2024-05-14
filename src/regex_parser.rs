use crate::regex_lexer::{RegexLexer, RegexToken};

#[derive(Debug)]
/// Node from the AST created by the parser
pub enum Node {
    Or {
        nodes: Vec<Node>,
    },
    Not {
        nodes: Vec<Node>,
    },
    Literal(char),
    StartAnchor,
    Digit,
    Alphanum,
    Wildcard,
    Group {
        nodes: Vec<Node>,
        group_ref: usize,
    },
    Quantifier {
        node: Box<Node>,
        min: usize,
        max: Option<usize>,
    },
    End {
        node: Box<Node>,
    },
}

pub struct RegexParser {
    l: RegexLexer,
    cur_token: RegexToken,
    peek_token: RegexToken,
    group_ref: usize,
}

impl RegexParser {
    pub fn new(lexer: RegexLexer) -> anyhow::Result<Self> {
        let mut parser = Self {
            l: lexer,
            cur_token: RegexToken::Eof,
            peek_token: RegexToken::Eof,
            group_ref: 0,
        };

        // sets cur and peek token
        parser.next_token()?;
        parser.next_token()?;
        Ok(parser)
    }

    pub fn next_token(&mut self) -> anyhow::Result<()> {
        self.cur_token = self.peek_token.clone();
        self.peek_token = self.l.next_token()?;
        Ok(())
    }

    /// For bracket we only match litterals
    pub fn build_bracket_group(&mut self) -> anyhow::Result<Node> {
        let mut nodes = Vec::new();

        let mut negated = false;

        if let RegexToken::StartAnchor = self.cur_token {
            self.next_token()?;
            negated = true;
        }

        loop {
            match self.cur_token {
                RegexToken::Literal(x) => {
                    nodes.push(Node::Literal(x));
                }
                RegexToken::RBracket => {
                    let final_node = if negated {
                        Node::Not { nodes }
                    } else {
                        Node::Or { nodes }
                    };

                    return Ok(final_node);
                }

                _ => todo!(),
            }
            self.next_token()?;
        }
    }

    pub fn build_ast(&mut self, group_ref: usize) -> anyhow::Result<Node> {
        let mut nodes = Vec::new();

        loop {
            match self.cur_token {
                RegexToken::Literal(x) => {
                    nodes.push(Node::Literal(x));
                }
                RegexToken::Digit => {
                    nodes.push(Node::Digit);
                }
                RegexToken::AlphaNum => {
                    nodes.push(Node::Alphanum);
                }
                RegexToken::Wildcard => {
                    nodes.push(Node::Wildcard);
                }
                RegexToken::StartAnchor => {
                    nodes.push(Node::StartAnchor);
                }
                RegexToken::EndAnchor => {
                    let last_node = nodes.pop().unwrap();
                    nodes.push(Node::End {
                        node: Box::new(last_node),
                    });
                }
                RegexToken::Quantifier { min, max } => {
                    let prev_node = nodes
                        .pop()
                        .ok_or_else(|| anyhow::anyhow!("Misplaced quantifier"))?;

                    let node = Node::Quantifier {
                        min,
                        max,
                        node: Box::new(prev_node),
                    };
                    nodes.push(node);
                }
                RegexToken::Pipe => {
                    self.next_token()?;
                    let left_node = Node::Group { nodes, group_ref };
                    let right_node = self.build_ast(group_ref)?;
                    nodes = vec![Node::Or {
                        nodes: vec![left_node, right_node],
                    }];
                }
                RegexToken::LBracket => {
                    self.next_token()?;
                    let node = self.build_bracket_group()?;
                    nodes.push(node);
                }
                RegexToken::LParen => {
                    self.group_ref += 1;
                    self.next_token()?;
                    let node = self.build_ast(self.group_ref)?;
                    nodes.push(node);
                }
                RegexToken::RParen => {
                    return Ok(Node::Group { nodes, group_ref });
                }
                RegexToken::Eof => {
                    return Ok(Node::Group {
                        nodes,
                        group_ref: 0,
                    });
                }
                _ => todo!(),
            }

            self.next_token()?;
        }
    }
}
