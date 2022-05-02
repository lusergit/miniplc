use log::trace;

use crate::{
    advance_with_expected,
    core::{
        ast::{ASTNode, FunctionCallNode, FunctionDeclNode, ReturnStmtNode},
        errors::SyntaxError,
        symbol_table::{Symbol, SymbolTable, SymbolType},
        token::Kind,
    },
    current_with_expected,
};

use super::Parser;

impl Parser {
    fn parse_single_param(&mut self) -> Result<Symbol, Vec<SyntaxError>> {
        trace!("parse single parameter");
        match self.advance().kind {
            Kind::Var => advance_with_expected!(Kind::Identifier, self, {
                let id = self.current.clone();
                advance_with_expected!(Kind::Colon, self, {
                    let ttype = self.parse_type()?;
                    match self.advance().kind {
                        Kind::Comma | Kind::RightParen => Ok(Symbol {
                            name: id.lexeme,
                            s_type: SymbolType::VarParam,
                            r_type: ttype,
                            position: id.position,
                        }),
                        other => Err(self.unexpected_token_err(Kind::RightParen, other)),
                    }
                })
            }),
            Kind::Identifier => {
                let id = self.current.clone();
                advance_with_expected!(Kind::Colon, self, {
                    let ttype = self.parse_type()?;
                    match self.advance().kind {
                        Kind::Comma | Kind::RightParen => Ok(Symbol {
                            name: id.lexeme,
                            s_type: SymbolType::Param,
                            r_type: ttype,
                            position: id.position,
                        }),
                        other => Err(self.unexpected_token_err(Kind::RightParen, other)),
                    }
                })
            }
            other => Err(self.unexpected_token_err(Kind::Identifier, other)),
        }
    }
    /// Proveate function to parse parameters in a function/procedure
    /// declaration
    pub fn parse_parameters(&mut self) -> Result<SymbolTable, Vec<SyntaxError>> {
        trace!("parse parameters");
        let mut params = SymbolTable::new();
        loop {
            let param = self.parse_single_param()?;
            params.push(param);
            if self.matches(Kind::RightParen) {
                break;
            }
        }
        Ok(params)
    }

    /// Parsing of a function block, returns either the corresponding
    /// ASTNode or a vector containing the found SyntaxErrors
    pub fn parse_function(&mut self) -> Result<ASTNode, Vec<SyntaxError>> {
        trace!("parse function");
        advance_with_expected!(Kind::Identifier, self, {
            let id = self.current.clone();
            advance_with_expected!(Kind::LeftParen, self, {
                let args = self.parse_parameters()?;
                current_with_expected!(
                    Kind::RightParen,
                    self,
                    advance_with_expected!(Kind::Colon, self, {
                        let r_type = self.parse_type()?;
                        advance_with_expected!(
                            Kind::Semicolon,
                            self,
                            advance_with_expected!(Kind::Begin, self, {
                                self.context.push(args.clone());
                                trace!("Parsing function {} block", id.lexeme);
                                let block = self.parse_block()?;
                                let mut ctx = self.context.pop().unwrap();
                                ctx.push(Symbol {
                                    name: id.lexeme.clone(),
                                    s_type: SymbolType::Function,
                                    r_type,
                                    position: id.position,
                                });
                                self.context.push(ctx);
                                Ok(ASTNode::FunctionDecl(FunctionDeclNode {
                                    name: id.lexeme,
                                    position: self.current.position,
                                    args,
                                    block: Box::new(block),
                                    r_type,
                                }))
                            })
                        )
                    })
                )
            })
        })
    }

    pub fn parse_call_parameters(&mut self) -> Result<SymbolTable, Vec<SyntaxError>> {
        trace!("parsing call parameters");
        let mut params = SymbolTable::new();
        let mut errors: Vec<SyntaxError> = Vec::new();
        trace!("Entering the loop");
        loop {
            match self.advance().kind {
                Kind::Identifier => {
                    let param = self.current.clone();
                    trace!("Single param: {}, {}", param, param.lexeme);
                    match self.context.last().unwrap().get(param.clone().lexeme) {
                        Some(p) => {
                            trace!("Symbol found!");
                            params.push(p);
                            match self.advance().kind {
                                Kind::Comma => {}
                                Kind::RightParen => {
                                    break;
                                }
                                Kind::Eof => {
                                    errors.push(self.error_at_current(
                                        "Found EOF wile parsing call parameters, missing a )?",
                                    ));
                                    break;
                                }
                                other => {
                                    trace!("Found other kind {}", other);
                                    errors.push(self.error_at_current(
                                        format!("Unexpected token: {}", other).as_str(),
                                    ));
                                }
                            }
                        }
                        None => {
                            trace!("Symbol {} not found", param.clone().lexeme);
                            errors.push(
                                self.error_at_current(
                                    format!("use of undeclared variable: {}", param.clone().lexeme)
                                        .as_str(),
                                ),
                            );
                        }
                    }
                }
                other => errors
                    .push(self.error_at_current(format!("Unexpected token: {}", other).as_str())),
            }
        }
        if errors.is_empty() {
            Ok(params)
        } else {
            Err(errors)
        }
    }

    pub fn parse_function_call(&mut self) -> Result<ASTNode, Vec<SyntaxError>> {
        trace!("parsing function call");
        let f_name = self.current.clone();
        advance_with_expected!(Kind::LeftParen, self, {
            let params = self.parse_call_parameters()?;
            trace!("Returned, current token: {}", self.current.clone());
            current_with_expected!(
                Kind::RightParen,
                self,
                Ok(ASTNode::FunctionCallStmt(FunctionCallNode {
                    position: f_name.position,
                    args: params,
                    target: f_name.lexeme,
                }))
            )
        })
    }

    pub fn parse_return(&mut self) -> Result<ASTNode, Vec<SyntaxError>> {
        let id = self.current.clone();
        match self.advance().kind {
            Kind::Semicolon => Ok(ASTNode::ReturnStmt(ReturnStmtNode {
                token: id,
                value: None,
            })),
            other => {
                trace!("other token: {}", other);
                self.go_back();
                let expr = self.parse_expression()?;
                current_with_expected!(
                    Kind::Semicolon,
                    self,
                    Ok(ASTNode::ReturnStmt(ReturnStmtNode {
                        token: id,
                        value: Some(Box::new(expr))
                    }))
                )
            }
        }
    }
}
