use crate::regex_parser::Node;
use std::collections::HashSet;

/// Struct that tries to match an input string to a pattern.
/// To do so, go through the whole AST (starting from root node),
/// and check if the node currently under examination matches one of the position in th
/// char vector. Note that quantifiers and Or can generate multiple potential paths,
/// which is why other matchers are spawned
/// Note to self: This is completely overkill
#[derive(Debug, Clone)]
pub struct Matcher {
    positions: HashSet<usize>,
}

impl Matcher {
    /// When creating a new matcher, we try to match starting all the positions in the
    /// char vec
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
            Node::StartAnchor => {
                if self.positions.contains(&0) {
                    self.positions.clear();
                    self.positions.insert(0);
                    true
                } else {
                    false
                }
            }
            Node::EndAnchor => {
                todo!()
            }
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{regex_lexer::RegexLexer, regex_parser::RegexParser};

    use super::*;

    #[rstest]
    #[case("(a(b))\\de\\w.f", "ab5e_%f", true)]
    #[case("(b|bc|de|fg)d45", "ded45h_", true)]
    #[case("ba?c+d{2,3}f*g", "bccdddffffffffg", true)]
    #[case("ba?c+d{2,3}f*g", "bccdffffffffg", false)]
    #[case("Ap[^pb]le", "Apple is good", false)]
    #[case("Ap[^ab]le", "Apple is good", true)]
    #[case("a.*b", "assgshgsoghsfohgsfoghsfghsgbe", true)]
    #[case("^aa(wz)?43", "aawz43xuy", true)]
    #[case("^(aa|bb)(ef)", "bbefg", true)]
    #[case("^(aa|bb)(ef)", " bbefg", false)]
    #[case("^aa", "baa", false)]
    // #[case("aa$", "aaaaab", false)]
    // #[case("aa$", "b(aa)a", true)]
    fn test_matcher(
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
