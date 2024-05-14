use std::collections::HashSet;

use crate::regex_lexer::{RegexLexer, RegexToken};

#[derive(Debug)]
enum Node {
    Or {
        nodes: Vec<Node>,
    },
    Not {
        node: Box<Node>,
    },
    Range(char, char),
    Literal(char),
    Digit,
    Alphanum,
    Wildcard,
    Group {
        nodes: Vec<Node>,
        group_ref: u8,
    },
    Quantifier {
        node: Box<Node>,
        min: u8,
        max: Option<u8>,
    },
}

#[derive(Debug, Clone)]
struct Matcher {
    positions: HashSet<usize>,
}

impl Matcher {
    pub fn new() -> Self {
        let mut positions = HashSet::new();
        positions.insert(0);
        Matcher { positions }
    }
    pub fn matches(&mut self, node_to_match: &Node, chars: &[char]) -> bool {
        match node_to_match {
            Node::Literal(c) => {
                let mut at_least_one_match = false;

                let mut new_positions = HashSet::new();
                for pos in self.positions.iter() {
                    let is_matching = *c == chars[*pos];
                    if is_matching {
                        new_positions.insert(*pos + 1);
                        at_least_one_match = true;
                    }
                }
                self.positions = new_positions;
                at_least_one_match
            }
            Node::Digit => {
                let mut at_least_one_match = false;

                let mut new_positions = HashSet::new();
                for pos in self.positions.iter() {
                    let c = chars[*pos];
                    let is_matching = c.is_ascii_digit();
                    if is_matching {
                        new_positions.insert(*pos + 1);
                        at_least_one_match = true;
                    }
                }
                self.positions = new_positions;
                at_least_one_match
            }
            Node::Alphanum => {
                let mut at_least_one_match = false;

                let mut new_positions = HashSet::new();
                for pos in self.positions.iter() {
                    let c = chars[*pos];
                    let is_matching = c.is_ascii_alphanumeric() || c == '_';
                    if is_matching {
                        new_positions.insert(*pos + 1);
                        at_least_one_match = true;
                    }
                }
                self.positions = new_positions;
                at_least_one_match
            }
            Node::Or { nodes } => {
                let matcher_clone = self.clone();
                let mut positions = HashSet::new();
                let mut at_least_one_match = false;
                for node in nodes {
                    let mut matcher = matcher_clone.clone();
                    if matcher.matches(node, chars) {
                        at_least_one_match = true;
                        for pos in matcher.positions {
                            positions.insert(pos);
                        }
                    }
                }
                self.positions = positions;
                at_least_one_match
            }
            Node::Group { nodes, group_ref } => {
                let mut is_matching = true;
                for (i, node) in nodes.iter().enumerate() {
                    if !self.matches(node, chars) {
                        is_matching = false;
                        break;
                    }
                }
                is_matching
            }
            _ => todo!(),
        }
    }
}

struct RegexParser {
    l: RegexLexer,
    cur_token: RegexToken,
    peek_token: RegexToken,
    group_ref: u8,
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
                    let mut final_node = Node::Or { nodes };
                    if negated {
                        final_node = Node::Not {
                            node: Box::new(final_node),
                        }
                    }

                    return Ok(final_node);
                }

                _ => todo!(),
            }
            self.next_token()?;
        }
    }

    pub fn build_ast(&mut self, group_ref: u8) -> anyhow::Result<Node> {
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_parser() -> anyhow::Result<()> {
        let pat = String::from("(a(b))\\de\\wf");
        let chars = "ab5e_f".chars().collect::<Vec<_>>();

        // let pat = String::from("(b|bc|de|fg)d45");
        // let chars = "fgd45".chars().collect::<Vec<_>>();

        let mut lexer = RegexLexer::new(&pat);
        let mut parser = RegexParser::new(lexer)?;

        let node = parser.build_ast(0)?;
        dbg!(&node);
        let mut matcher = Matcher::new();
        let b = matcher.matches(&node, &chars);
        assert!(b);

        Ok(())
    }
}
