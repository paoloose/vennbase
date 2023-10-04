use logic_parser::lexing::Lexer;
use logic_parser::parsing::{Parser, ASTNode};
use logic_parser::errors::{LexerError, ParserError};

pub fn parse_query(query: &str) -> logic_parser::parsing::Result<ASTNode> {
    let mut lexer = Lexer::with_alphabets(
        |c| c.is_alphanumeric() || c == '_' || c == ':' || c == '*' || c == '/',
        |c| c.is_alphabetic(),
    );

    let tokens = lexer.tokenize(query).map_err(|e| <LexerError as Into<ParserError>>::into(e))?;

    let mut parser = Parser::new(&tokens);
    parser.parse()
}
