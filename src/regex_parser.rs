use std::collections::HashSet;

use crate::regex_lexer::{RegexLexer, RegexToken};

#[derive(Debug)]
pub enum Node {
    Or {
        nodes: Vec<Node>,
    },
    Not {
        nodes: Vec<Node>,
    },
    Range(char, char),
    Literal(char),
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
}

#[derive(Debug, Clone)]
pub struct Matcher {
    positions: HashSet<usize>,
}

impl Matcher {
    pub fn new(len_char: usize) -> Self {
        let mut positions = HashSet::new();
        for pos in 0..len_char {
            positions.insert(pos);
        }
        Matcher { positions }
    }
    pub fn matches(&mut self, node_to_match: &Node, chars: &[char]) -> bool {
        self.positions = self
            .positions
            .clone()
            .into_iter()
            .filter(|&x| x < chars.len())
            .collect();
        match node_to_match {
            Node::Wildcard => {
                let mut new_positions = HashSet::new();
                for pos in self.positions.iter() {
                    new_positions.insert(*pos + 1);
                }
                self.positions = new_positions;
                true
            }
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
            // should only contain literal nodes
            Node::Not { nodes } => {
                let mut chars_not_to_match = HashSet::new();
                for node in nodes {
                    match node {
                        Node::Literal(x) => {
                            chars_not_to_match.insert(*x);
                        }
                        _ => todo!(),
                    }
                }

                let mut at_least_one_match = false;

                let mut new_positions = HashSet::new();
                for pos in self.positions.iter() {
                    let is_matching = !chars_not_to_match.contains(&chars[*pos]);
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
            Node::Quantifier { node, min, max } => {
                let mut positions = HashSet::new();
                let mut at_least_one_match = false;
                let mut min = *min;
                if min == 0 {
                    positions.extend(self.positions.clone());
                    at_least_one_match = true;
                    min = 1;
                }

                let max = match max {
                    Some(max) => *max,
                    None => {
                        let min_pos = *self.positions.iter().min().unwrap_or(&0);
                        chars.len() - min_pos + 1
                    }
                };

                let mut nb_match = 0;
                let mut matcher = self.clone();
                while nb_match < max {
                    let is_matching = matcher.matches(node, chars);
                    if is_matching {
                        nb_match += 1;
                        if nb_match >= min {
                            at_least_one_match = true;
                            positions.extend(matcher.positions.clone());
                        }
                    } else {
                        break;
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

    #[rstest]
    #[case("(a(b))\\de\\w.f", "ab5e_%f", true)]
    #[case("(b|bc|de|fg)d45", "ded45h_", true)]
    #[case("ba?c+d{2,3}f*g", "bccdddffffffffg", true)]
    #[case("ba?c+d{2,3}f*g", "bccdffffffffg", false)]
    #[case("Ap[^pb]le", "Apple is good", false)]
    #[case("Ap[^ab]le", "Apple is good", true)]
    #[case("a.*b", "assgshgsoghsfohgsfoghsfghsgbe", true)]
    fn test_parser(
        #[case] pat: &str,
        #[case] input: &str,
        #[case] expected: bool,
    ) -> anyhow::Result<()> {
        let pat = pat.to_string();
        let chars = input.chars().collect::<Vec<_>>();

        let lexer = RegexLexer::new(&pat);
        let mut parser = RegexParser::new(lexer)?;

        let node = parser.build_ast(0)?;
        dbg!(&node);
        let mut matcher = Matcher::new(chars.len());
        let is_match = matcher.matches(&node, &chars);
        assert_eq!(is_match, expected);

        Ok(())
    }
}
