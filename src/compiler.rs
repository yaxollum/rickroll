use lazy_static::lazy_static;
use regex::Regex;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::error::*;
use crate::lexer::*;
use crate::util::*;

#[derive(Debug, EnumIter)]
pub enum Statement {
    Say,
    Let,
    Assign,
}

impl Statement {
    pub fn matches(&self, raw: &String) -> bool {
        lazy_static! {
            // print
            static ref SAY: Regex = Regex::new("^Never gonna say .+$").unwrap();
            // let + assign to var
            static ref LET: Regex = Regex::new("^Never gonna let \\w+ down$").unwrap();
            static ref ASSIGN: Regex = Regex::new("^Never gonna give \\w+ .+$").unwrap();
        }
        use Statement::*;
        return match self {
            Say => &(*SAY),
            Let => &(*LET),
            Assign => &(*ASSIGN),
        }
        .is_match(raw);
    }

    pub fn match_statement(raw: &String) -> Option<Statement> {
        for statement in Statement::iter() {
            if statement.matches(raw) {
                return Some(statement);
            }
        }
        return None;
    }
}

#[derive(Debug)]
pub struct Compiler {
    ptr: usize,
    raw: Vec<String>,
    global_scope: Scope,
}

impl Compiler {
    pub fn new(raw_txt: String) -> Compiler {
        Compiler {
            ptr: 0,
            raw: {
                let mut res = Vec::new();
                let mut cur = String::new();
                for chr in raw_txt.chars() {
                    if chr == '\r' || chr == '\n' {
                        res.push(cur);
                        cur = String::new();
                    } else {
                        cur.push(chr);
                    }
                }
                res.push(cur);
                res
            },
            global_scope: Scope::new(),
        }
    }

    fn advance(&mut self) {
        self.ptr += 1;
    }

    // wraps a traceback around a possible error
    fn wrap_check<T>(&self, res: Result<T, Error>) -> Result<T, Error> {
        if let Err(error) = res {
            return Err(Error::traceback(error, Some(self.ptr + 1)));
        }
        return res;
    }

    // Vec<(original line number, instruction)>
    // instructions with no original line or expected error should have a line number of 0
    pub fn compile(mut self) -> Result<Vec<(usize, Instruction)>, Error> {
        let mut compiled: Vec<(usize, Instruction)> = Vec::new();
        while self.ptr < self.raw.len() {
            // try to match a statement
            let curln = self.raw[self.ptr].trim();
            if curln != "" {
                let res = Statement::match_statement(&String::from(curln));
                // no statement matched
                if res.is_none() {
                    return Err(Error::new(
                        ErrorType::SyntaxError,
                        "Illegal statement",
                        Some(self.ptr + 1),
                    ));
                }
                // compile statement to bytecode
                use Instruction::*;
                use Statement::*;
                match res.unwrap() {
                    Say => {
                        // ^Never gonna say .+$
                        let expr = String::from(&curln[16..]);
                        let tokens = self.wrap_check(
                            Lexer::new(expr, self.global_scope.clone()).make_tokens(),
                        )?;
                        // push Put instruction
                        compiled.push((self.ptr + 1, Put(tokens)));
                    }
                    Statement::Let => {
                        // ^Never gonna let \\w+ down$
                        let varname = String::from(&curln[16..(curln.len() - 5)]);
                        println!("debug new var: {}", varname);
                        if self.global_scope.has_var(varname.clone()) {
                            return Err(Error::new(
                                ErrorType::NameError,
                                &(format!(
                                    "Variable {} already exists in the current scope",
                                    varname
                                ))[..],
                                Some(self.ptr + 1),
                            ));
                        }
                        self.global_scope.add_var(varname.clone());
                        // push Let instruction
                        compiled.push((self.ptr + 1, Instruction::Let(varname)));
                    }
                    Assign => {
                        // ^Never gonna give \\w+ .+$
                        let slice = String::from(&curln[17..]); // \\w .+
                        match slice.find(' ') {
                            Some(index) => {
                                let varname = String::from(String::from(&slice[..index]).trim());
                                let expr = String::from(&slice[(index + 1)..]);
                                let tokens = self.wrap_check(
                                    Lexer::new(expr, self.global_scope.clone()).make_tokens(),
                                )?;
                                println!("debug assign var: {} to {:?}", varname, tokens);
                                // push Set instruction
                                compiled.push((self.ptr + 1, Set(varname, tokens)));
                            }
                            None => {
                                return Err(Error::new(
                                    ErrorType::SyntaxError,
                                    "Illegal statement",
                                    Some(self.ptr + 1),
                                ))
                            }
                        }
                    }
                }
            }
            // advance
            self.advance();
        }
        compiled.push((0, Instruction::End()));
        return Ok(compiled);
    }
}
