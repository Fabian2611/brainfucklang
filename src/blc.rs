use std::collections::HashMap;

fn line_contains_expr(s: &&str) -> bool {
    !s.replace(char::is_whitespace, "").is_empty() && !s.starts_with("--")
}

fn strip_inline_comment(s: &str) -> &str {
    if let Some(l) = s.split_once("--") {
        l.0
    } else {
        s
    }
}

fn ensure_valid_var_name<'a>(var_space: &Vec<VarType>, s: &'a str) -> &'a str {
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
    {
        for v in var_space {
            match v {
                VarType::Var(n) => {
                    if *n == s {
                        panic!("Duplicate variable name: {}", s);
                    }
                }
                VarType::Arr(n, _) => {
                    if *n == s {
                        panic!("Duplicate variable name: {}", s);
                    }
                }
            }
        }
        s
    } else {
        panic!("Invalid variable name: {}", s);
    }
}

fn position_of_var_expr(_: bool, s: &str, map: &HashMap<&str, VarAllocation>, line: &str) -> usize {
    if s.contains("#") {
        let (v, i) = s.split_once('#').unwrap();
        let i = i
            .parse::<usize>()
            .map_err(|_| panic!("Invalid line '{}': '{}' is not a valid index", line, i))
            .unwrap();
        if !map.contains_key(v) {
            panic!("Invalid line '{}': unknown arr {}", line, s)
        }
        let al = map.get(v).unwrap();
        if let VarAllocation::Arr(p, l) = *al {
            if i >= l {
                panic!(
                    "Invalid line '{}': arr index {} out of range {}",
                    line, i, p
                )
            };
            p + i
        } else {
            panic!(
                "Invalid line '{}': {} is a variable, cannot be indexed",
                line, s
            )
        }
    } else {
        if !map.contains_key(s) {
            panic!("Invalid line '{}': unknown var {}", line, s)
        }
        let al = map.get(s).unwrap();
        if let VarAllocation::Var(p) = *al {
            p
        } else {
            panic!(
                "Invalid line '{}': {} is an array, cannot be passed as an argument",
                line, s
            )
        }
    }
}

fn parse_byte(val: &str, line: &str) -> u8 {
    val.parse::<u8>()
        .map_err(|_| {
            panic!(
                "Invalid line '{}': '{}' is not a valid byte value",
                line, val
            )
        })
        .unwrap()
}

fn parse_char(val: &str, line: &str) -> u8 {
    val.parse::<char>()
        .map_err(|_| {
            panic!(
                "Invalid line '{}': '{}' is not a valid byte value",
                line, val
            )
        })
        .unwrap() as u8
}

fn build_var_map<'a>(
    var_space: &Vec<VarType<'a>>,
    var_map: &mut HashMap<&'a str, VarAllocation>,
    var_space_size: &mut usize,
) {
    let mut o = 4; // cells 0 to 3 are registers
    for v in var_space {
        match v {
            VarType::Var(n) => {
                var_map.insert(n, VarAllocation::Var(o));
                o += 1;
            }
            VarType::Arr(n, s) => {
                var_map.insert(n, VarAllocation::Arr(o, *s));
                o += *s;
            }
        }
        *var_space_size = o;
    }
}

fn push_instr<'a>(
    current_macro: &mut Option<(&'a str, Vec<&'a str>, Vec<Instruction>)>,
    instructions: &mut Vec<Instruction>,
    instr: Instruction,
) {
    if let Some((_, _, body)) = current_macro {
        body.push(instr);
    } else {
        instructions.push(instr);
    }
}

pub fn parse(code: String) -> (String, usize) {
    let mut data_mode = true;
    let mut macro_mode = false;
    let mut var_space: Vec<VarType> = Vec::new();
    let mut var_space_size: usize = 0;
    let mut var_map: HashMap<&str, VarAllocation> = HashMap::new();

    var_map.insert("__ra", VarAllocation::Var(0));
    var_map.insert("__rb", VarAllocation::Var(1));
    var_map.insert("__rc", VarAllocation::Var(2));
    var_map.insert("__rd", VarAllocation::Var(3));

    let mut instructions: Vec<Instruction> = Vec::new();
    let mut macros: HashMap<&str, (Vec<&str>, Vec<Instruction>)> = HashMap::new();
    let mut current_macro: Option<(&str, Vec<&str>, Vec<Instruction>)> = None;
    let mut macro_while_depth: usize = 0;

    for line in code
        .split(|c| c == '\n' || c == ';')
        .map(strip_inline_comment)
        .filter(line_contains_expr)
        .map(|l| l.trim())
    {
        if line == ".data" {
            macro_mode = false;
            continue;
        } else if line == ".text" {
            if data_mode {
                data_mode = false;
                build_var_map(&var_space, &mut var_map, &mut var_space_size);
            }
            macro_mode = false;
            continue;
        } else if line == ".macros" {
            if data_mode {
                data_mode = false;
                build_var_map(&var_space, &mut var_map, &mut var_space_size);
            }
            macro_mode = true;
            continue;
        }

        if data_mode {
            let s = line.split_whitespace().collect::<Vec<_>>();
            if s.len() <= 1 {
                panic!("Invalid line '{}': not a valid data expression", line)
            }
            match *s.get(0).unwrap() {
                "var" => {
                    if s.len() != 2 {
                        panic!(
                            "Invalid line '{}': not a valid var expression - too many args",
                            line
                        )
                    }
                    var_space.push(VarType::Var(ensure_valid_var_name(
                        &var_space,
                        *s.get(1).unwrap(),
                    )))
                }
                "arr" => {
                    if s.len() == 2 {
                        panic!(
                            "Invalid line '{}': not a valid arr expression - too few args",
                            line
                        )
                    }
                    if s.len() != 3 {
                        panic!(
                            "Invalid line '{}': not a valid arr expression - too many args",
                            line
                        )
                    }
                    var_space.push(VarType::Arr(
                        ensure_valid_var_name(&var_space, *s.get(1).unwrap()),
                        s.get(2)
                            .unwrap()
                            .parse::<usize>()
                            .map_err(|_| {
                                panic!("Invalid line '{}': last arg is not a valid size", line);
                            })
                            .map(|v| {
                                if v == 0 {
                                    panic!("Invalid line '{}': arr size may not be 0", line);
                                }
                                v
                            })
                            .unwrap(),
                    ))
                }
                _ => panic!("Invalid line '{}': not a valid data expression", line),
            }
        } else {
            let s = line.split_whitespace().collect::<Vec<_>>();
            if s.is_empty() {
                panic!("Invalid line '{}': empty expression", line);
            }
            let instr = match *s.get(0).unwrap() {
                "mac" => {
                    if !macro_mode {
                        panic!(
                            "Invalid line '{}': may not define macros outside of .macros",
                            line
                        );
                    }
                    if s.len() <= 1 {
                        panic!(
                            "Invalid line '{}': not a valid mac expression - requires 1 or more args",
                            line
                        );
                    }
                    if current_macro.is_some() {
                        panic!(
                            "Invalid line '{}': nested macro definitions are not allowed",
                            line
                        );
                    }
                    let name = *s.get(1).unwrap();
                    if macros.contains_key(name) {
                        panic!("Invalid line '{}': duplicate macro name: {}", line, name);
                    }
                    let args: Vec<&str> = s[2..].to_vec();
                    for &arg in &args {
                        let n = ensure_valid_var_name(&var_space, arg);
                        var_space.push(VarType::Var(n));
                        var_map.insert(n, VarAllocation::Var(var_space_size));
                        var_space_size += 1;
                    }
                    current_macro = Some((name, args, Vec::new()));
                    macro_while_depth = 0;
                    continue;
                }
                "call" => {
                    if s.len() <= 1 {
                        panic!(
                            "Invalid line '{}': not a valid call expression - requires 1 or more args",
                            line
                        );
                    }
                    let name = *s.get(1).unwrap();
                    let args: Vec<&str> = s[2..].to_vec();
                    let macro_def = macros.get(name).unwrap_or_else(|| {
                        panic!("Invalid line '{}': unknown macro '{}'", line, name)
                    });
                    let params = macro_def.0.clone();
                    let body = macro_def.1.clone();
                    if params.len() != args.len() {
                        panic!(
                            "Invalid line '{}': macro '{}' expects {} argument(s), got {}",
                            line,
                            name,
                            params.len(),
                            args.len()
                        );
                    }
                    let mut expansion: Vec<Instruction> = Vec::new();
                    for (param, arg) in params.into_iter().zip(args.into_iter()) {
                        let pos = position_of_var_expr(macro_mode, param, &var_map, line);
                        if param.starts_with('_') {
                            let arg_pos = position_of_var_expr(macro_mode, arg, &var_map, line);
                            expansion.push(Instruction::Let(pos, arg_pos));
                        } else if param.starts_with('$') {
                            expansion.push(Instruction::Set(pos, parse_byte(arg, line)));
                        } else {
                            panic!(
                                "Invalid line '{}': macro parameter '{}' must start with '_' or '$'",
                                line, param
                            );
                        }
                    }
                    expansion.extend(body);
                    for instr in expansion {
                        push_instr(&mut current_macro, &mut instructions, instr);
                    }
                    continue;
                }
                "set" => {
                    if s.len() != 3 {
                        panic!(
                            "Invalid line '{}': not a valid set expression - requires 2 args",
                            line
                        );
                    }
                    let v = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    let val = *s.get(2).unwrap();
                    Instruction::Set(v, parse_byte(val, line))
                }
                "setchar" => {
                    if s.len() != 3 {
                        panic!(
                            "Invalid line '{}': not a valid setchar expression - requires 2 args",
                            line
                        );
                    }
                    let v = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    let val = *s.get(2).unwrap();
                    Instruction::Set(v, parse_char(val, line))
                }
                "let" => {
                    if s.len() != 3 {
                        panic!(
                            "Invalid line '{}': not a valid let expression - requires 2 args",
                            line
                        );
                    }
                    let v1 = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    let v2 = position_of_var_expr(macro_mode, *s.get(2).unwrap(), &var_map, line);
                    Instruction::Let(v1, v2)
                }
                "inc" => {
                    if s.len() != 3 {
                        panic!(
                            "Invalid line '{}': not a valid inc expression - requires 2 args",
                            line
                        );
                    }
                    let v = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    let val = *s.get(2).unwrap();
                    Instruction::Inc(v, parse_byte(val, line))
                }
                "dec" => {
                    if s.len() != 3 {
                        panic!(
                            "Invalid line '{}': not a valid inc expression - requires 2 args",
                            line
                        );
                    }
                    let v = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    let val = *s.get(2).unwrap();
                    Instruction::Dec(v, parse_byte(val, line))
                }
                "add" => {
                    if s.len() != 3 {
                        panic!(
                            "Invalid line '{}': not a valid add expression - requires 2 args",
                            line
                        );
                    }
                    let v1 = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    let v2 = position_of_var_expr(macro_mode, *s.get(2).unwrap(), &var_map, line);
                    Instruction::Add(v1, v2)
                }
                "sub" => {
                    if s.len() != 3 {
                        panic!(
                            "Invalid line '{}': not a valid sub expression - requires 2 args",
                            line
                        );
                    }
                    let v1 = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    let v2 = position_of_var_expr(macro_mode, *s.get(2).unwrap(), &var_map, line);
                    Instruction::Sub(v1, v2)
                }
                "while" => {
                    if s.len() != 2 {
                        panic!(
                            "Invalid line '{}': not a valid while expression - requires 1 arg",
                            line
                        );
                    }
                    let v = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    if current_macro.is_some() {
                        macro_while_depth += 1;
                    }
                    Instruction::While(v)
                }
                "end" => {
                    if s.len() != 1 {
                        panic!("Invalid line '{}': end does not take arguments", line);
                    }
                    if current_macro.is_some() {
                        if macro_while_depth == 0 {
                            let (name, params, body) = current_macro.take().unwrap();
                            macros.insert(name, (params, body));
                            continue;
                        } else {
                            macro_while_depth -= 1;
                        }
                    }
                    Instruction::End
                }
                "dbg" => {
                    if s.len() != 1 {
                        panic!("Invalid line '{}': dbg does not take arguments", line);
                    }
                    Instruction::Dbg
                }
                "print" => {
                    if s.len() != 2 {
                        panic!(
                            "Invalid line '{}': not a valid print expression - requires 1 arg",
                            line
                        );
                    }
                    let v = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    Instruction::Print(v)
                }
                "input" => {
                    if s.len() != 2 {
                        panic!(
                            "Invalid line '{}': not a valid input expression - requires 1 arg",
                            line
                        );
                    }
                    let v = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    Instruction::Input(v)
                }
                "halt" => {
                    if s.len() != 1 {
                        panic!("Invalid line '{}': halt does not take arguments", line);
                    }
                    Instruction::Halt
                },
                "mul" => {
                    if s.len() != 3 {
                        panic!(
                            "Invalid line '{}': not a valid mul expression - requires 2 args",
                            line
                        );
                    }
                    let v1 = position_of_var_expr(macro_mode, *s.get(1).unwrap(), &var_map, line);
                    let v2 = position_of_var_expr(macro_mode, *s.get(2).unwrap(), &var_map, line);
                    Instruction::Mul(v1, v2)
                }
                _ => panic!("Invalid line '{}': unknown instruction", line),
            };

            push_instr(&mut current_macro, &mut instructions, instr);
        }
    }

    println!("{:?}", var_map);
    println!("{:?}", instructions);

    (to_bf(instructions), var_space_size)
}

fn calculate_offset(dp: usize, c: usize) -> isize {
    c as isize - dp as isize
}

fn mv(code: &mut String, amt: isize, dp: usize) -> usize {
    if amt > 0 {
        for _ in 0..amt {
            code.push('>');
        }
    } else if amt < 0 {
        for _ in 0..amt.abs() {
            code.push('<');
        }
    }
    (dp as isize + amt) as usize
}

fn setzero(code: &mut String) {
    code.push_str("[-]");
}

fn add_immediate(code: &mut String, v: u8) {
    for _ in 0..v {
        code.push('+');
    }
}

fn to_bf(instructions: Vec<Instruction>) -> String {
    let mut code = String::new();
    let mut dp: usize = 0;
    let mut loop_stack: Vec<usize> = Vec::new();

    for instr in instructions {
        match instr {
            Instruction::Set(c, 0) => {
                code.push_str(format!(" set({};0): ", c).as_str());
                let offset = calculate_offset(dp, c);
                dp = mv(&mut code, offset, dp);
                setzero(&mut code);
            }
            Instruction::Set(c, v) => {
                code.push_str(format!(" set({};{}): ", c, v).as_str());
                let offset = calculate_offset(dp, c);
                dp = mv(&mut code, offset, dp);
                setzero(&mut code);
                add_immediate(&mut code, v);
            }
            Instruction::Let(c1, c2) => {
                code.push_str(format!(" let({};{}): ", c1, c2).as_str());
                let offset_c2_to_c1 = calculate_offset(c2, c1);
                let offset_c1_to_0 = -(c1 as isize);
                dp = mv(&mut code, calculate_offset(dp, c1), dp); // now at c1
                setzero(&mut code);
                dp = mv(&mut code, offset_c1_to_0, dp); // now at 0
                setzero(&mut code);
                dp = mv(&mut code, c2 as isize, dp); // now at c2
                code.push_str("[-"); // start loop and dec c2
                dp = mv(&mut code, offset_c2_to_c1, dp); // then move to and inc c1
                code.push('+');
                dp = mv(&mut code, offset_c1_to_0, dp); // then move to and inc 0
                code.push('+');
                dp = mv(&mut code, c2 as isize, dp); // then move back to c2
                code.push(']'); // end loop
                dp = mv(&mut code, -(c2 as isize), dp); // now at 0
                code.push_str("[-"); // start a new loop and dec 0
                dp = mv(&mut code, c2 as isize, dp); // then move to and inc c2
                code.push('+');
                dp = mv(&mut code, -(c2 as isize), dp); // then move back to 0
                code.push(']'); // end loop
            }
            Instruction::Inc(c, v) => {
                code.push_str(format!(" inc({};{}): ", c, v).as_str());
                let offset = calculate_offset(dp, c);
                dp = mv(&mut code, offset, dp);
                for _ in 0..v {
                    code.push('+');
                }
            }
            Instruction::Dec(c, v) => {
                code.push_str(format!(" dec({};{}): ", c, v).as_str());
                let offset = calculate_offset(dp, c);
                dp = mv(&mut code, offset, dp);
                for _ in 0..v {
                    code.push('-');
                }
            }
            Instruction::Add(c1, c2) => {
                code.push_str(format!(" add({};{}): ", c1, c2).as_str());
                let offset_c2_to_c1 = calculate_offset(c2, c1);
                dp = mv(&mut code, calculate_offset(dp, c2), dp); // now at c2
                code.push_str("[-"); // start loop and dec c2
                dp = mv(&mut code, offset_c2_to_c1, dp); // move to c1
                code.push('+'); // inc c1
                dp = mv(&mut code, -offset_c2_to_c1, dp); // move to c2
                code.push(']'); // end loop
            }
            Instruction::Sub(c1, c2) => {
                code.push_str(format!(" add({};{}): ", c1, c2).as_str());
                let offset_c2_to_c1 = calculate_offset(c2, c1);
                dp = mv(&mut code, calculate_offset(dp, c2), dp); // now at c2
                code.push_str("[-"); // start loop and dec c2
                dp = mv(&mut code, offset_c2_to_c1, dp); // move to c1
                code.push('-'); // dec c1
                dp = mv(&mut code, -offset_c2_to_c1, dp); // move to c2
                code.push(']'); // end loop
            }
            Instruction::While(c) => {
                code.push_str(format!(" while({}): ", c).as_str());
                let offset = calculate_offset(dp, c);
                dp = mv(&mut code, offset, dp);
                code.push_str("[");
                loop_stack.push(c);
            }
            Instruction::End => {
                let c = match loop_stack.pop() {
                    Some(c) => c,
                    None => panic!("End statement without matching while statement"),
                };
                code.push_str(format!(" endwhile({}): ", c).as_str());
                let offset = calculate_offset(dp, c);
                dp = mv(&mut code, offset, dp);
                code.push_str("]");
            }
            Instruction::Dbg => {
                code.push('@');
            }
            Instruction::Print(c) => {
                code.push_str(format!(" print({}): ", c).as_str());
                let offset = calculate_offset(dp, c);
                dp = mv(&mut code, offset, dp);
                code.push_str(".");
            }
            Instruction::Input(c) => {
                code.push_str(format!(" input({}): ", c).as_str());
                let offset = calculate_offset(dp, c);
                dp = mv(&mut code, offset, dp);
                code.push_str(",");
            }
            Instruction::Halt=> {
                code.push_str(" halt: ");
                dp = mv(&mut code, -(dp as isize), dp); // move to c0
                setzero(&mut code);
                code.push('+'); // set c0 to 1 so the loop executes
                code.push_str("[]");
            }
            Instruction::Mul(c1, c2) => {
                code.push_str(format!(" mul({};{}): ", c1, c2).as_str());
                let offset_c2_to_c1 = calculate_offset(c2, c1);
                let offset_c1_to_rb = -(c1 as isize) + 1; // rb is idx 1
                dp = mv(&mut code, calculate_offset(dp, c1), dp); // now at c1

                // move c1 into ra and rb
                dp = mv(&mut code, offset_c1_to_rb, dp); // now at rb
                setzero(&mut code);
                dp = mv(&mut code, -1, dp); // now at ra
                setzero(&mut code);

                dp = mv(&mut code, c1 as isize, dp); // now at c1
                code.push_str("[-"); // start loop and dec c1
                dp = mv(&mut code, offset_c1_to_rb, dp); // now at rb
                code.push('+'); // inc rb
                dp = mv(&mut code, -1, dp); // now at ra
                code.push_str("+"); // inc ra
                dp = mv(&mut code, c1 as isize, dp); // move back to c1
                code.push_str("]"); // end loop

                // multiply with c2 as counter
                dp = mv(&mut code, -offset_c2_to_c1, dp); // now at c2
                code.push_str("[-"); // start mul loop and dec c2

                // add ra to c1 and rc
                dp = mv(&mut code, -(c2 as isize), dp); // now at ra
                code.push_str("[-"); // start add loop and dec ra
                dp = mv(&mut code, c1 as isize, dp); // now at c1
                code.push_str("+"); // inc c1
                dp = mv(&mut code, calculate_offset(c1, 2), dp); // move to rc
                code.push_str("+"); // inc rc
                dp = mv(&mut code, -2, dp); // move back to ra
                code.push_str("]"); // end add loop

                // restore rc into ra
                dp = mv(&mut code, 2, dp); // now at rc
                code.push_str("[-"); // start loop and dec rc
                dp = mv(&mut code, -2, dp); // now at ra
                code.push_str("+"); // inc ra
                dp = mv(&mut code, 2, dp); // move back to rc
                code.push_str("]"); // end loop

                dp = mv(&mut code, calculate_offset(2, c2), dp); // move back to c2
                code.push_str("]"); // end mul loop
            }
            Instruction::Div(c1, c2) => {
                code.push_str(format!(" div({};{}): ", c1, c2).as_str());

                dp = mv(&mut code, -(dp as isize), dp); // now at ra
                setzero(&mut code);
                code.push('+'); // ra is now 1
                dp = mv(&mut code, c2 as isize, dp); // now at c2

                // ensure c2 neq 0, else halt
                code.push_str("["); // this loop is only entered if c2 neq 0
                // move c2 to rb to avoid infinite loop
                code.push_str("[-"); // start inner loop and dec c2
                dp = mv(&mut code, -(c2 as isize) + 1, dp); // now at rb
                code.push_str("+"); // inc rb
                dp = mv(&mut code, c2 as isize - 1, dp); // move back to c2
                code.push_str("]"); // end inner loop;
                // set ra to zero
                dp = mv(&mut code, -(c2 as isize), dp); // now at ra
                setzero(&mut code);
                dp = mv(&mut code, c2 as isize, dp); // move back to c2
                code.push_str("]"); // end loop
                // go to ra and enter infinite loop if it is still 1
                dp = mv(&mut code, -(c2 as isize), dp); // now at ra
                code.push_str("[]"); // halt if ra neq 0
                // move rb back into c2
                dp = mv(&mut code, 1, dp); // now at rb
                code.push_str("[-"); // begin loop and dec rb
                dp = mv(&mut code, (c2 as isize) - 1, dp); // now at c2
                code.push_str("+"); // inc c2
                dp = mv(&mut code, -(c2 as isize) + 1, dp); // move back to rb
                code.push_str("]"); // end loop

                // TODO
            }
        }
    }

    if !loop_stack.is_empty() {
        panic!("Unclosed while statement");
    }

    code
}

#[derive(Debug)]
enum VarType<'a> {
    Var(&'a str),
    Arr(&'a str, usize),
}

#[derive(Debug)]
enum VarAllocation {
    Var(usize),
    Arr(usize, usize),
}

#[derive(Debug, Clone)]
enum Instruction {
    Set(usize, u8),
    Let(usize, usize),
    Inc(usize, u8),
    Dec(usize, u8),
    Add(usize, usize),
    Sub(usize, usize),
    While(usize),
    End,
    Dbg,
    Print(usize),
    Input(usize),
    Halt,
    Mul(usize, usize),
    Div(usize, usize),
}
