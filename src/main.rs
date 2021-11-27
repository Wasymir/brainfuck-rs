use crossterm::{
    event::{read, Event, KeyCode},
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode},
    ExecutableCommand,
};
use std::{fs::read_to_string, io::stdout};
use clap::{Arg, App, ArgGroup};


#[derive(Debug)]
enum Token {
    Increment,
    Decrement,
    IncrementPointer,
    DecrementPointer,
    Output,
    Input,
    EnterLoop,
    ExitLoop,
}

#[derive(Debug)]
enum AstNode {
    Increment,
    Decrement,
    IncrementPointer,
    DecrementPointer,
    Output,
    Input,
    Loop(Vec<AstNode>),
}

#[derive(Debug)]
enum Error {
    FileOpenError,
    MissingLoopDelimiter,
    MissingLoopOpening,
    MemoryPointerOverflow,
    NonAsciiOutputValue,
}

fn lexer(keys: Vec<char>) -> Vec<Token> {
    let mut tokens = Vec::new();
    for key in keys.iter() {
        tokens.push(match key {
            '>' => Token::IncrementPointer,
            '<' => Token::DecrementPointer,
            '+' => Token::Increment,
            '-' => Token::Decrement,
            '.' => Token::Output,
            ',' => Token::Input,
            '[' => Token::EnterLoop,
            ']' => Token::ExitLoop,
            _ => {
                continue;
            }
        });
    }
    return tokens;
}

fn parser(tokens: Vec<Token>) -> Result<Vec<AstNode>, Error> {
    let mut ast = Vec::new();
    let mut loop_counter = 0;
    for token in tokens.iter() {
        {
            let mut current_ast_branch = &mut ast;
            for _ in 0..loop_counter {
                current_ast_branch = match current_ast_branch.iter_mut().last().unwrap() {
                    AstNode::Loop(l) => l,
                    _ => panic!("Unexpected node"),
                }
            }
            current_ast_branch
        }
        .push(match token {
            Token::IncrementPointer => AstNode::IncrementPointer,
            Token::DecrementPointer => AstNode::DecrementPointer,
            Token::Increment => AstNode::Increment,
            Token::Decrement => AstNode::Decrement,
            Token::Input => AstNode::Input,
            Token::Output => AstNode::Output,
            Token::EnterLoop => {
                loop_counter += 1;
                AstNode::Loop(Vec::new())
            }
            Token::ExitLoop => {
                loop_counter -= 1;
                if loop_counter < 0 {
                    return Err(Error::MissingLoopOpening);
                } else {
                    continue;
                }
            }
        });
    }
    if loop_counter != 0 {
        return Err(Error::MissingLoopDelimiter);
    }
    return Ok(ast);
}

struct RuntimeEnvironment {
    mem: [u8; 30_000],
    pointer: usize,
}
impl RuntimeEnvironment {
    fn new() -> Self {
        RuntimeEnvironment {
            mem: [0u8; 30_000],
            pointer: 0usize,
        }
    }
    fn c_mem(&self) -> &u8 {
        &self.mem[self.pointer]
    }
    fn c_mem_mut(&mut self) -> &mut u8 {
        &mut self.mem[self.pointer]
    }
    fn run(&mut self, ast: &Vec<AstNode>) -> Result<(), Error> {
        for node in ast.iter() {
            match node {
                AstNode::IncrementPointer => {
                    self.pointer += 1usize;
                    if self.pointer > 30_000usize {
                        return Err(Error::MemoryPointerOverflow);
                    }
                }
                AstNode::DecrementPointer => {
                    if let Some(v) = self.pointer.checked_sub(1usize) {
                        self.pointer = v;
                    } else {
                        return Err(Error::MemoryPointerOverflow);
                    }
                }
                AstNode::Increment => {
                    *self.c_mem_mut() = self.c_mem().wrapping_add(1u8);
                }
                AstNode::Decrement => {
                    *self.c_mem_mut() = self.c_mem().wrapping_sub(1u8);
                }
                AstNode::Output => {
                    if self.c_mem().is_ascii() {
                        stdout().execute(Print(*self.c_mem() as char)).unwrap();
                    } else {
                        return Err(Error::NonAsciiOutputValue);
                    }
                }
                AstNode::Input => {
                    enable_raw_mode().unwrap();
                    loop {
                        if let Event::Key(k) = read().unwrap() {
                            if let KeyCode::Char(c) = k.code {
                                if c.is_ascii() {
                                    *self.c_mem_mut() = c as u8;
                                    disable_raw_mode().unwrap();
                                    break;
                                }
                            }
                        }
                    }
                }
                AstNode::Loop(l) => loop {
                    if self.c_mem() != &0u8 {
                        self.run(l)?;
                    } else {
                        break;
                    }
                },
            }
        }
        return Ok(());
    }
}

fn main() -> Result<(),Error> {
   let matches = App::new("Rust Brainfuck Interpreter")
       .version("1.0")
       .author("Wasymir")
       .about("Just another Brainfuck interpreter")
       .arg(Arg::with_name("INPUT")
           .help("path to file to run")
           .index(1))
       .arg(Arg::with_name("code")
           .short("c")
           .long("code")
           .takes_value(true)
           .help("runs code passed")
           .value_name("CODE")
       )
       .group(ArgGroup::with_name("sources")
           .args(&["INPUT", "code"])
           .required(true)
        )
       .get_matches();
    let keys: Vec<char> = {
        if let Some(p) = matches.value_of("INPUT") {
            if let Ok(s) = read_to_string(p) {
                s.chars().collect()
            } else {
                return Err(Error::FileOpenError);
            }
        } else {
            matches.value_of("code").unwrap().chars().collect()
        } 
    };
    let tokens = lexer(keys);
    let ast = parser(tokens)?;
    let mut rtenv = RuntimeEnvironment::new();
    rtenv.run(&ast)?;
    Ok(())
}
