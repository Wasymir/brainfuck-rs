use crossterm::{
    event::{read, Event, KeyCode},
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode},
    ExecutableCommand,
};
use std::{env::args, fmt, fs::read_to_string, io::stdout};

enum Token {
    IncrementPointer,
    DecrementPointer,
    Increment,
    Decrement,
    Output,
    Input,
    StartLoop,
    EndLoop,
}

#[derive(Debug)]
enum ASTNode {
    IncrementPointer,
    DecrementPointer,
    Increment,
    Decrement,
    Output,
    Input,
    Loop(Vec<ASTNode>),
}

#[derive(Debug)]
enum Error {
    MissingLoopDelimiter,
    MissingLoopOpening,
    MemValueOverflow,
    MemPointerOverflow,
    NonAsciiValue,
    FileOpenError,
}
impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "{}",
            match self {
                Self::MissingLoopDelimiter => "Parser Error: Missing Loop Delimiter",
                Self::MissingLoopOpening => "Parser Error: Missing Loop Opening",
                Self::MemValueOverflow => "Runtime Error: Memory Value Overflow",
                Self::MemPointerOverflow => "Runtime Error: Memory Pointer Overflow",
                Self::NonAsciiValue => "Runtime Error: Tried To Output Non-Ascii Value",
                Self::FileOpenError => "Couldn't open file",
            }
        )
    }
}

fn lexer(source: Vec<char>) -> Vec<Token> {
    let mut tokens = Vec::new();
    for key in source.into_iter() {
        if ['<', '>', '-', '+', '[', ']', ',', '.'].contains(&key) {
            tokens.push(match key {
                '<' => Token::DecrementPointer,
                '>' => Token::IncrementPointer,
                '-' => Token::Decrement,
                '+' => Token::Increment,
                '.' => Token::Output,
                ',' => Token::Input,
                '[' => Token::StartLoop,
                ']' => Token::EndLoop,
                _ => panic!("unexpected token"),
            });
        }
    }
    return tokens;
}

fn parser(tokens: Vec<Token>) -> Result<Vec<ASTNode>, Error> {
    let mut ast = Vec::new();
    let mut in_loop = false;
    for token in tokens.into_iter() {
        if let Token::EndLoop = token {
            if !in_loop {
                return Err(Error::MissingLoopOpening);
            }
            in_loop = false;
        } else {
            {
                if in_loop {
                    match ast.last_mut().unwrap() {
                        ASTNode::Loop(e) => e,
                        _ => panic!("Expected Loop Node"),
                    }
                } else {
                    &mut ast
                }
            }
            .push(match token {
                Token::DecrementPointer => ASTNode::DecrementPointer,
                Token::IncrementPointer => ASTNode::IncrementPointer,
                Token::Decrement => ASTNode::Decrement,
                Token::Increment => ASTNode::Increment,
                Token::Output => ASTNode::Output,
                Token::Input => ASTNode::Input,
                Token::StartLoop => {
                    in_loop = true;
                    ASTNode::Loop(Vec::new())
                }
                _ => panic!("Unexpected Token"),
            });
        }
    }
    if in_loop {
        return Err(Error::MissingLoopDelimiter);
    }
    return Ok(ast);
}

struct Interpreter {
    mem: [u8; 30_000],
    pointer: usize,
}
impl Interpreter {
    fn new() -> Self {
        Interpreter {
            mem: [0u8; 30_000],
            pointer: 0usize,
        }
    }
    fn run_ast_node(&mut self, node: &ASTNode) -> Result<(), Error> {
        match node {
            ASTNode::DecrementPointer => match self.pointer.checked_sub(1usize) {
                Some(p) => {
                    self.pointer = p;
                    Ok(())
                }
                None => Err(Error::MemPointerOverflow),
            },
            ASTNode::IncrementPointer => match self.pointer.checked_add(1usize) {
                Some(p) => {
                    self.pointer = p;
                    Ok(())
                }
                None => Err(Error::MemPointerOverflow),
            },
            ASTNode::Decrement => match self.mem[self.pointer].checked_sub(1u8) {
                Some(v) => {
                    self.mem[self.pointer] = v;
                    Ok(())
                }
                None => Err(Error::MemValueOverflow),
            },
            ASTNode::Increment => match self.mem[self.pointer].checked_add(1u8) {
                Some(v) => {
                    self.mem[self.pointer] = v;
                    Ok(())
                }
                None => Err(Error::MemValueOverflow),
            },
            ASTNode::Input => {
                loop {
                    match read().unwrap() {
                        Event::Key(key) => match key.code {
                            KeyCode::Char(c) => {
                                if c.is_ascii() {
                                    self.mem[self.pointer] = c as u8;
                                    break;
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
                Ok(())
            }
            ASTNode::Output => {
                if self.mem[self.pointer].is_ascii() {
                    stdout().execute(Print(self.mem[self.pointer] as char));
                    Ok(())
                } else {
                    Err(Error::NonAsciiValue)
                }
            }
            ASTNode::Loop(nodes) => {
                loop {
                    for loop_node in nodes.into_iter() {
                        self.run_ast_node(loop_node);
                    }
                    if self.mem[self.pointer] == 0u8 {
                        break;
                    }
                }
                Ok(())
            }
        }
    }
}
fn main() -> Result<(), Error> {
    enable_raw_mode().unwrap();
    let path = &args().collect::<Vec<String>>()[1];
    match read_to_string(&path) {
        Ok(s) => {
            let tokens = lexer(s.chars().collect());
            let ast = parser(tokens)?;
            let mut interpreter = Interpreter::new();
            for (idx, node) in ast.iter().enumerate() {
                match interpreter.run_ast_node(node) {
                    Ok(_) => {}
                    Err(e) => {
                        stdout().execute(Print(format!(
                            "Error at node: {:?}, index: {:?}",
                            node, idx
                        )));
                        return Err(e);
                    }
                }
            }
        }
        Err(e) => {
            if path.is_empty() {
                stdout().execute(Print("Please enter path to file as argument"));
                return Ok(());
            } else {
                print!("{:?}", e);
                return Err(Error::FileOpenError);
            }
        }
    }
    disable_raw_mode().unwrap();
    return Ok(());
}
