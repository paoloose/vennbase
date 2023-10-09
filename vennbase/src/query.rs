use std::collections::HashMap;

use logic_parser::lexing::Lexer;
use logic_parser::parsing::{Parser, ASTNode};
use logic_parser::errors::{LexerError, ParserError};

pub fn parse_query(query: &str) -> logic_parser::parsing::Result<ASTNode> {
    let mut lexer = Lexer::with_alphabets(
        |c| c.is_alphanumeric() || c == '_' || c == '-' || c == ':' || c == '*' || c == '/',
        |c| c.is_alphabetic(),
    );

    let tokens = lexer.tokenize(query).map_err(|e| <LexerError as Into<ParserError>>::into(e))?;

    let mut parser = Parser::new(&tokens);
    parser.parse()
}

// FIXME: fix this on upstream for `logic-parser`
// (mime:image/* && tag:anime) || (mime:video/* && !tag:anime)
pub fn get_variables(_ast: &ASTNode) -> Vec<String> {
    vec!["mime:image/*".into(), "tag:anime".into(), "mime:video/*".into(), "tag:anime".into()]
}

// This enum differentiates between fixed-value propositions and fickle ones
pub enum PropositionType {
    Fixed(bool),
    Fickle,
}

/// An iterator that, given a set of propositions, it permutates
/// all the possible boolean values for the set.
///
/// The advantage of this is that you can mark variables as fixed,
/// so you will only get the permutations of the propositions that
/// are marked as fickle.
pub struct VariablesPermutations<'a> {
    variables: &'a [PropositionType],
    // how many propositions are marked as Fixed
    fickles_powers: Vec<usize>,
    i: usize,
    max_i: usize,
}

use PropositionType::{Fixed, Fickle};

impl<'a> VariablesPermutations<'a> {
    pub fn new(variables: &'a [PropositionType]) -> Self {
        let fickles = variables.iter().filter(|v| matches!(v, Fickle)).collect::<Vec<&PropositionType>>();
        VariablesPermutations {
            variables,
            i: 0,
            fickles_powers: fickles
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    // If, for example, we have the variables
                    //   [Fickle(a), Fickle(b), Fixed(true), Fickle(c)]
                    // this will generate the powers for the three fickle variables
                    //   [a: 4, b: 2, c: 1]
                    let p = usize::pow(2, fickles.len() as u32 - i as u32 - 1);
                    dbg!(p);
                    p
                }).collect(),
            max_i: usize::pow(2, fickles.len() as u32),
        }
    }
}

impl<'a> Iterator for VariablesPermutations<'a> {
    type Item = Vec<bool>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.max_i {
            return None;
        }

        let mut fickle_i = 0;
        // This will generate a new permutation for the fickle variables
        // according to the current iteration self.i
        let evaluations = self.variables.iter().enumerate().map(|(_, variable)| {
            match variable {
                // If the proposition is fixed, just return it
                Fixed(value) => *value,
                Fickle => {
                    let value = (self.i / self.fickles_powers[fickle_i]) % 2;
                    fickle_i += 1;
                    value != 0
                },
            }
        }).collect::<Vec<bool>>();

        self.i += 1;
        Some(evaluations)
    }
}

pub fn evaluate(tree: &ASTNode, values: &HashMap<String, bool>) -> Result<bool, ()> {
    match tree {
        ASTNode::Not { operand } => {
            Ok(!evaluate(operand, values)?)
        },
        ASTNode::And { left, right } => {
            Ok(evaluate(left, values)? && evaluate(right, values)?)
        },
        ASTNode::Or { left, right } => {
            Ok(evaluate(left, values)? || evaluate(right, values)?)
        },
        ASTNode::Implies { left, right } => {
            Ok(!evaluate(left, values)? || evaluate(right, values)?)
        },
        ASTNode::IfAndOnlyIf { left, right } => {
            Ok(evaluate(left, values)? == evaluate(right, values)?)
        },
        ASTNode::Literal { value } => {
            Ok(*value)
        },
        ASTNode::Identifier { name } => {
            Ok(*values.get(name).unwrap())
        },
    }
}
