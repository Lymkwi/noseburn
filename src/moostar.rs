//! Runner for the moostar visualizer
use std::collections::{HashMap, VecDeque};
use std::iter::Peekable;
use std::error::Error;

#[derive(Debug, Clone)]
pub struct MooError {
    message: String
}

impl MooError {
    fn new(msg: &str) -> Self {
        Self { message: msg.into() }
    }
}

impl std::fmt::Display for MooError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for MooError {}

#[derive(Clone, Debug)]
enum MooInst {
    Plus,
    Minus,
    Left,
    Right,
    In,
    Out,
    OpenLoop,
    CloseLoop,
    Nop(char),
    Call(usize),
    FuncStart(usize),
    FuncEnd(usize),
    MetaJump,
    Halt
}

fn fetch_identifier(chars: &mut Peekable<std::str::Chars<'_>>) -> Result<(String, usize), Box<dyn Error>> {
    let mut identifier: String = String::new();
    let mut eaten: usize = 0;
    let mut stage: usize = 0; // 0 ws before, 1 eat identifier, 2 ws after
    while chars.peek().is_some() {
        let c = chars.peek().unwrap();
        match stage {
            0 => {
                if !c.is_whitespace() {
                    stage += 1;
                    // Is it a lower ascii ?
                    if c.is_ascii_lowercase() {
                        continue;
                    }
                    return Err(Box::new(MooError::new("First character in identifier is not lowercase")));
                }
            },
            1 => {
                if c.is_ascii_alphanumeric() || *c == '_' {
                    identifier.push(*c);
                } else {
                    stage += 1;
                    continue;
                }
            },
            2 => {
                if !c.is_whitespace() { break; }
            },
            _ => unreachable!()
        }
        let _ = chars.next().unwrap();
        eaten += 1;
    }
    if identifier.is_empty() {
        return Err(Box::new(MooError::new("Empty identifier")));
    }
    Ok((identifier, eaten))
}

type SpannedInstruction = (MooInst, (usize, usize));
type MethodIndex = HashMap<usize, usize>;

pub struct Runner {
    /// Stack of iteration/call return pointers
    return_positions: VecDeque<usize>,
    /// Pointers
    pointer: usize,
    meta_pointer: usize,
    is_meta: bool,
    /// Ribbons
    data_ribbon: HashMap<usize, u8>,
    meta_ribbon: HashMap<usize, u8>,
    /// Program
    program: Vec<SpannedInstruction>,
    instruction_pointer: usize,
    halted: bool,
    /// Input
    input: String,
    /// Output,
    output: String,
    /// Method management
    method_index: MethodIndex,
}

impl Runner {
    pub fn new(program: &str) -> Result<Self, Box<dyn Error>> {
        // Process
        let (instr, method_index) = Self::process(program)?;
        // Find the index of the first non-defining instruction
        let mut silencer: bool = true;
        let mut idx: usize = 0;
        for (pos, inst) in instr.iter().enumerate() {
            match inst.0 {
                MooInst::FuncStart(_) => { silencer = true; },
                MooInst::FuncEnd(_) => { silencer = false; },
                MooInst::Nop(_) => {},
                _ => { if !silencer { idx = pos; break; } }
            }
        }
        Ok(Self {
            return_positions: VecDeque::new(),
            pointer: 0,
            meta_pointer: 0,
            is_meta: false,
            halted: false,
            data_ribbon: HashMap::new(),
            meta_ribbon: HashMap::new(),
            program: instr,
            instruction_pointer: idx,
            input: String::new(),
            output: String::new(),
            method_index
        })
    }

    fn process(program: &str) -> Result<(Vec<SpannedInstruction>, MethodIndex), Box<dyn Error>> {
        let mut method_lookup: HashMap<String, usize> = HashMap::new();
        let mut method_index: MethodIndex = HashMap::new();
        let mut program_out: Vec<(MooInst, (usize, usize))> = Vec::new();
        let mut method_fetcher = program.chars().peekable();
        let mut pos = 0;
        let mut current_function_definition: Option<usize> = None;
        while method_fetcher.peek().is_some() {
            let c = method_fetcher.next().unwrap();
            match c {
                '+' => { program_out.push((MooInst::Plus, (pos, 1))); },
                '-' => { program_out.push((MooInst::Minus, (pos, 1))); },
                '>' => { program_out.push((MooInst::Right, (pos, 1))); },
                '<' => { program_out.push((MooInst::Left, (pos, 1))); },
                '.' => { program_out.push((MooInst::Out, (pos, 1))); },
                ',' => { program_out.push((MooInst::In, (pos, 1))); },
                '[' => { program_out.push((MooInst::OpenLoop, (pos, 1))); },
                ']' => { program_out.push((MooInst::CloseLoop, (pos, 1))); },
                '(' => {
                    // If we are in a definition we can't re-define
                    if current_function_definition.is_some() {
                        return Err(Box::new(MooError::new("Re-opening definition in one another")));
                    }
                    // Fetch the name of the definition
                    let (ident, eaten) = fetch_identifier(&mut method_fetcher)?;
                    //println!("Found ident {} (peek {:?})", ident, method_fetcher.peek());
                    // Check the closing paren, colon and opening bracket
                    for guard in [')', ':', '{'] {
                        if method_fetcher.next() != Some(guard) {
                            return Err(Box::new(MooError::new(&format!("Expected '{}' after function declaration header", guard))));
                        }
                    }

                    // Do we know about the function ?
                    let function_code = match method_lookup.get(&ident) {
                        Some(&c) => c,
                        None => {
                            method_lookup.insert(ident, method_lookup.len());
                            method_lookup.len()-1
                        }
                    };
                    // Insert a code
                    method_index.insert(function_code, program_out.len());
                    program_out.push((MooInst::FuncStart(function_code), (pos, 4 + eaten)));
                    current_function_definition = Some(function_code);
                    pos += 3 + eaten;
                },
                '}' => {
                    if let Some(current) = current_function_definition {
                        program_out.push((MooInst::FuncEnd(current), (pos, 1)));
                        current_function_definition = None;
                    } else {
                        return Err(Box::new(MooError::new("Ended an unknown function definition")));
                    }
                },
                '~' => {
                    // Fetch the name of the function
                    let (ident, eaten) = fetch_identifier(&mut method_fetcher)?;
                    // Check the semicolon
                    if method_fetcher.next() != Some(';') {
                        return Err(Box::new(MooError::new("Expected ';' after function call identifier")));
                    }
                    // Get the identifier
                    let method_code = match method_lookup.get(&ident) {
                        Some(&c) => c,
                        None => {
                            let n = method_lookup.len();
                            method_lookup.insert(ident, n);
                            n
                        }
                    };
                    // Insert the call
                    program_out.push((MooInst::Call(method_code), (pos, eaten + 2)));
                    pos += 1 + eaten;
                },
                '^' => { program_out.push((MooInst::MetaJump, (pos, 1))); },
                c => { program_out.push((MooInst::Nop(c), (pos, 1))); }
            }
            pos += 1;
        }

        program_out.push((MooInst::Halt, (pos, 1)));
        Ok((program_out, method_index))
    }

    /// Getters and setters
    pub fn get_input(&self) -> &str {
        &self.input
    }

    pub fn get_output(&self) -> &str {
        &self.output
    }

    pub fn jump_list(&self, max_of: Option<usize>) -> Vec<usize> {
        self.return_positions
            .iter()
            .take(max_of.unwrap_or(self.return_positions.len()))
            .copied()
            .collect::<Vec<usize>>()
    }

    /// Obtain the span of the next instruction to be executed
    pub fn get_instruction_span(&self) -> (usize, usize) {
        self.next_instruction().1
    }

    fn next_instruction(&self) -> &(MooInst, (usize, usize)) {
        self.program.get(self.instruction_pointer).unwrap()
    }

    fn save_pointer(&mut self) {
        self.return_positions.push_front(self.instruction_pointer);
    }

    fn retrieve_pointer(&mut self) -> usize {
        self.return_positions.pop_front().unwrap()
    }

    pub fn get_value(&self) -> u8 {
        if self.is_meta {
            *self.meta_ribbon.get(&self.meta_pointer).unwrap_or(&0)
        } else {
            *self.data_ribbon.get(&self.pointer).unwrap_or(&0)
        }
    }

    pub fn plus(&mut self) {
        if self.is_meta {
            let res = self.meta_ribbon.entry(self.meta_pointer).or_insert(0).wrapping_add(1);
            self.meta_ribbon.insert(self.meta_pointer, res);
        } else {
            let res = self.data_ribbon.entry(self.pointer).or_insert(0).wrapping_add(1);
            self.data_ribbon.insert(self.pointer, res);
        }
    }

    pub fn minus(&mut self) {
        if self.is_meta {
            let res = self.meta_ribbon.entry(self.meta_pointer).or_insert(0).wrapping_sub(1);
            self.meta_ribbon.insert(self.meta_pointer, res);
        } else {
            let res = self.data_ribbon.entry(self.pointer).or_insert(0).wrapping_sub(1);
            self.data_ribbon.insert(self.pointer, res);
        }
    }

    pub fn step(&mut self) {
        loop {
            // Look at where we are
            let (instr, _) = self.next_instruction();
            match instr {
                MooInst::Halt => { self.halted = true; },
                MooInst::Plus => {
                    self.plus();
                    self.instruction_pointer += 1;
                },
                MooInst::Minus => {
                    self.minus();
                    self.instruction_pointer += 1;
                },
                MooInst::Left => {
                    if self.is_meta {
                        self.meta_pointer -= 1;
                    } else {
                        self.pointer -= 1;
                    }
                    self.instruction_pointer += 1;
                },
                MooInst::Right => {
                    if self.is_meta {
                        self.meta_pointer += 1;
                    } else {
                        self.pointer += 1;
                    }
                    self.instruction_pointer += 1;
                },
                MooInst::OpenLoop => {
                    // Evaluate the current value
                    let value = self.get_value();
                    if value == 0 {
                        // Find the next close bracket
                        let mut varen: usize = 1;
                        while varen > 0 {
                            self.instruction_pointer += 1;
                            match self.next_instruction().0 {
                                MooInst::OpenLoop => { varen += 1; },
                                MooInst::CloseLoop => { varen -= 1; },
                                _ => {}
                            }
                        }
                        self.instruction_pointer += 1;
                    } else {
                        // Push the value to memory
                        self.save_pointer();
                        // Move once
                        self.instruction_pointer += 1;
                    }
                },
                MooInst::CloseLoop => {
                    // Move back to the opening of the loop
                    self.instruction_pointer = self.retrieve_pointer();
                },
                MooInst::Out
                | MooInst::In => {
                    panic!("NIY")
                },
                MooInst::Call(n) => {
                    // Find the function position
                    let position = *self.method_index.get(n).unwrap();
                    // Save the current position + 1 to jump back
                    self.save_pointer();
                    // Jump
                    self.instruction_pointer = position;
                },
                MooInst::FuncStart(_n) => {
                    self.instruction_pointer += 1;
                },
                MooInst::FuncEnd(_) => {
                    // Pop the pointer back
                    let position = self.retrieve_pointer();
                    self.instruction_pointer = position + 1;
                },
                MooInst::Nop(_c) => {
                    // Move one and continue
                    self.instruction_pointer += 1;
                    continue;
                },
                MooInst::MetaJump => {
                    self.is_meta = !self.is_meta;
                    self.instruction_pointer += 1;
                }
            }    
            break;
        }

        // Move forward as long as it's a Nop
        while let MooInst::Nop(_) = self.next_instruction().0 {
            self.instruction_pointer += 1;
        }
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }
}
