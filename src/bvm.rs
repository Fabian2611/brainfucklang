use std::io::Read;

pub struct BrainfuckVM {
    code: Vec<Instruction>,
    jumps: Vec<usize>,
    tape: Vec<u8>,
    ip: usize,
    dp: usize,
}

impl BrainfuckVM {
    pub fn new(code: String, size: usize) -> Self {
        let code = Self::parse(code);
        Self {
            jumps: Self::build_jumps(&code),
            code,
            tape: vec![0; size],
            ip: 0,
            dp: 0,
        }
    }

    pub fn get(&self, addr: usize) -> u8 {
        *self.tape.get(addr).expect("Index out of bounds")
    }

    pub fn set(&mut self, addr: usize, val: u8) {
        *self.tape.get_mut(addr).expect("Index out of bounds") = val;
    }

    pub fn run(&mut self) {
        let mut entered_loop = false;
        while self.ip < self.code.len() {
            entered_loop = self.step(entered_loop);
        }
    }

    pub fn step(&mut self, entered_loop_prev: bool) -> bool {
        let instr = &self.code[self.ip];
        let mut entered_loop = false;
        match instr {
            Instruction::PtrInc => self.dp += 1,
            Instruction::PtrDec => self.dp -= 1,
            Instruction::DataInc => self.tape[self.dp] = self.tape[self.dp].wrapping_add_signed(1),
            Instruction::DataDec => self.tape[self.dp] = self.tape[self.dp].wrapping_sub_signed(1),
            Instruction::Print => {
                use std::io::Write;
                print!("{}", self.tape[self.dp] as char);
                std::io::stdout().flush().expect("flush failed");
            },
            Instruction::Input => {
                use std::io::Write;
                std::io::stdout().flush().expect("flush failed");
                let mut buf = [0u8];
                match std::io::stdin().read(&mut buf) {
                    Ok(0) => self.tape[self.dp] = 0,
                    Ok(_) => self.tape[self.dp] = buf[0],
                    Err(e) => panic!("Input read failed: {e}"),
                }
            },
            Instruction::LpOpen => {
                if self.tape[self.dp] == 0 {
                    self.ip = self.jumps[self.ip];
                } else {
                    entered_loop = true;
                }
            },
            Instruction::LpClose => {
                if self.tape[self.dp] != 0 {
                    self.ip = self.jumps[self.ip];
                    if entered_loop_prev { // infinite loop!
                        println!("\nExecution halted.");
                        std::process::exit(0);
                    }
                }
            },
            Instruction::DbgPrint => self.print_tape(),
        }
        self.ip += 1;
        entered_loop
    }

    fn print_tape(&self) {
        let n = self.tape.len();
        let mut parts: Vec<String> = Vec::new();
        let mut i = 0;

        while i < n {
            let val = self.tape[i];
            if val == 0 {
                let start = i;
                let mut end = i;
                while end < n && self.tape[end] == 0 {
                    end += 1;
                }
                let run_len = end - start;
                let dp_in_run = self.dp >= start && self.dp < end;

                if run_len > 9 && !dp_in_run {
                    for k in start..start + 5 {
                        parts.push(self.cell_str(k));
                    }
                    parts.push("<...>".to_string());
                } else {
                    for k in start..end {
                        parts.push(self.cell_str(k));
                    }
                }
                i = end;
            } else {
                parts.push(self.cell_str(i));
                i += 1;
            }
        }

        println!("[{}]", parts.join(" "));
    }

    fn cell_str(&self, idx: usize) -> String {
        if idx == self.dp {
            format!("*{}*", self.tape[idx])
        } else {
            self.tape[idx].to_string()
        }
    }

    fn build_jumps(code: &[Instruction]) -> Vec<usize> {
        let mut jumps = vec![0usize; code.len()];
        let mut stack = Vec::new();
        for (i, instr) in code.iter().enumerate() {
            match instr {
                Instruction::LpOpen => stack.push(i),
                Instruction::LpClose => {
                    let open = stack.pop().expect("unmatched ']'");
                    jumps[open] = i;
                    jumps[i] = open;
                }
                _ => {}
            }
        }
        assert!(stack.is_empty(), "unmatched '['");
        jumps
    }

    fn parse(code: String) -> Vec<Instruction> {
        let mut instr = Vec::new();
        for ch in code.chars() {
            match ch {
                '>' => Some(Instruction::PtrInc),
                '<' => Some(Instruction::PtrDec),
                '+' => Some(Instruction::DataInc),
                '-' => Some(Instruction::DataDec),
                '.' => Some(Instruction::Print),
                ',' => Some(Instruction::Input),
                '[' => Some(Instruction::LpOpen),
                ']' => Some(Instruction::LpClose),
                '@' => Some(Instruction::DbgPrint),
                _ => None,
            }.map(|i| instr.push(i));
        };
        instr
    }
}

enum Instruction {
    PtrInc,
    PtrDec,
    DataInc,
    DataDec,
    Print,
    Input,
    LpOpen,
    LpClose,
    DbgPrint,
}
