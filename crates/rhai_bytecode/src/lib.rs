use rhai::{Expr, Stmt};
pub use rhai;

static COMPILE_ENGINE: std::sync::LazyLock<rhai::Engine> =
    std::sync::LazyLock::new(|| rhai::Engine::new_raw());

#[cfg(feature = "size16")]
pub type OpSize = u16;
#[cfg(feature = "size32")]
pub type OpSize = u32;
#[cfg(feature = "size64")]
pub type OpSize = u64;

pub type INT = rhai::INT;
pub type FLOAT = rhai::FLOAT;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ByteCode {
    DynamicConstant(rhai::Dynamic),
    UnitConstant,
    BoolConstant(bool),
    IntegerConstant(INT),
    FloatConstant(FLOAT),
    CharConstant(char),
    StringConstant(String),
    InterpolatedString(OpSize),
    ConstructArray(OpSize),
    Variable(OpSize),
    FnCall(OpSize, OpSize),
    Jump(OpSize),
    JumpIfTrue(OpSize),
    JumpIfFalse(OpSize),
    JumpIfNotNull(OpSize),
    VarInit(OpSize),
    Index,
    Return,
    PopStack,
}

pub trait DynamicValue: Sized + Clone {
    fn from_dynamic(dynamic: rhai::Dynamic) -> anyhow::Result<Self>;
    fn from_unit() -> anyhow::Result<Self> {
        return Self::from_dynamic(rhai::Dynamic::UNIT);
    }
    fn from_bool(v: bool) -> anyhow::Result<Self> {
        return Self::from_dynamic(rhai::Dynamic::from(v));
    }
    fn from_integer(v: INT) -> anyhow::Result<Self> {
        return Self::from_dynamic(rhai::Dynamic::from(v));
    }
    fn from_float(v: FLOAT) -> anyhow::Result<Self> {
        return Self::from_dynamic(rhai::Dynamic::from(v));
    }
    fn from_char(v: char) -> anyhow::Result<Self> {
        return Self::from_dynamic(rhai::Dynamic::from(v));
    }
    fn from_string(v: &String) -> anyhow::Result<Self> {
        return Self::from_dynamic(rhai::Dynamic::from(v.clone()));
    }
    fn from_variable_ref(var_id: OpSize) -> anyhow::Result<Self>;
    fn from_variable_element_ref(var_id: OpSize, indexes: Vec<OpSize>) -> anyhow::Result<Self>;
    fn is_unit(&self, variables: &Vec<Self>) -> anyhow::Result<bool>;
    fn to_bool(&self, variables: &Vec<Self>) -> anyhow::Result<bool>;
    fn to_size(&self, variables: &Vec<Self>) -> anyhow::Result<OpSize>;
    fn deref<'a>(&'a self, variables: &'a Vec<Self>) -> anyhow::Result<&'a Self>;
    fn deref_mut<'a>(&self, variables: &'a mut Vec<Self>) -> anyhow::Result<&'a mut Self>;
    fn ref_append_index(&mut self, ind: OpSize) -> anyhow::Result<()>;
}

pub struct Executer<T: DynamicValue> {
    fn_names: Vec<String>,
    fns: Vec<Box<dyn Fn(Vec<T>, &mut Vec<T>) -> anyhow::Result<T>>>,
}

impl<T: DynamicValue> Executer<T> {
    pub fn new() -> Self {
        return Self {
            fn_names: vec![],
            fns: vec![],
        };
    }
    fn function_names(&self) -> &Vec<String> {
        return &self.fn_names;
    }
    pub fn add_fn(
        &mut self,
        name: String,
        func: Box<dyn Fn(Vec<T>, &mut Vec<T>) -> anyhow::Result<T>>,
    ) -> anyhow::Result<()> {
        if self.fn_names.contains(&name) {
            anyhow::bail!("Function \"{}\" already exists!", name);
        } else {
            self.fns.push(func);
            self.fn_names.push(name);
            return Ok(());
        }
    }
    fn call_fn(&self, index: OpSize, args: Vec<T>, variables: &mut Vec<T>) -> anyhow::Result<T> {
        let ind = index as usize;
        if ind >= self.fns.len() {
            anyhow::bail!("Function #{} does not exist!", index);
        } else {
            return self.fns[ind](args, variables);
        }
    }
}

fn find_index(vec: &Vec<String>, name: &str, type_str: &str) -> anyhow::Result<OpSize> {
    match vec.iter().position(|x| x == name) {
        Some(i) => {
            return Ok(i as OpSize);
        }
        None => {
            anyhow::bail!("Undefined {} \"{}\"!", type_str, name);
        }
    }
}

fn find_index_auto_append(vec: &mut Vec<String>, name: &str) -> OpSize {
    match vec.iter().position(|x| x == name) {
        Some(i) => {
            return i as OpSize;
        }
        None => {
            vec.push(name.to_string());
            return (vec.len() - 1) as OpSize;
        }
    }
}

fn append_expr(
    functions: &Vec<String>,
    variables: &mut Vec<String>,
    break_pos: &mut Vec<usize>,
    continue_pos: &mut Vec<usize>,
    byte_codes: &mut Vec<ByteCode>,
    expr: &Expr,
) -> anyhow::Result<()> {
    match expr {
        Expr::DynamicConstant(dynamic, _) => {
            byte_codes.push(ByteCode::DynamicConstant(*dynamic.clone()));
        }
        Expr::BoolConstant(v, _) => {
            byte_codes.push(ByteCode::BoolConstant(*v));
        }
        Expr::IntegerConstant(v, _) => {
            byte_codes.push(ByteCode::IntegerConstant(*v));
        }
        Expr::FloatConstant(float_wrapper, _) => {
            byte_codes.push(ByteCode::FloatConstant(*float_wrapper.as_ref()));
        }
        Expr::CharConstant(v, _) => {
            byte_codes.push(ByteCode::CharConstant(*v));
        }
        Expr::StringConstant(immutable_string, _) => {
            byte_codes.push(ByteCode::StringConstant(immutable_string.to_string()));
        }
        Expr::InterpolatedString(thin_vec, _) => {
            for expr in thin_vec {
                append_expr(
                    functions,
                    variables,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    expr,
                )?;
            }
            byte_codes.push(ByteCode::InterpolatedString(thin_vec.len() as OpSize));
        }
        Expr::Array(thin_vec, _) => {
            for sub_expr in thin_vec {
                append_expr(
                    functions,
                    variables,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_expr,
                )?;
            }
        }
        Expr::Map(..) => {
            anyhow::bail!("Map not supported yet!");
        }
        Expr::Unit(..) => {
            byte_codes.push(ByteCode::UnitConstant);
        }
        Expr::Variable(data, _, _) => {
            let var_id = find_index(variables, data.1.as_str(), "variable")?;
            byte_codes.push(ByteCode::Variable(var_id));
        }
        Expr::ThisPtr(..) => {
            anyhow::bail!("\"this\" pointer not supported yet!");
        }
        Expr::Property(..) => {
            anyhow::bail!("Property not supported yet!");
        }
        Expr::MethodCall(..) => {
            anyhow::bail!("Method not supported yet!");
        }
        Expr::Stmt(stmt_block) => {
            for stmt in stmt_block.iter() {
                append_stmt(
                    functions,
                    variables,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    stmt,
                )?;
            }
        }
        Expr::FnCall(fn_call_expr, _) => {
            for sub_expr in &fn_call_expr.args {
                append_expr(
                    functions,
                    variables,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_expr,
                )?;
            }
            let fn_id = find_index(functions, fn_call_expr.name.as_str(), "function")?;
            byte_codes.push(ByteCode::FnCall(fn_id, fn_call_expr.args.len() as OpSize));
        }
        Expr::Dot(..) => {
            anyhow::bail!("Dot operator (.) not supported yet!");
        }
        Expr::Index(binary_expr, astflags, _) => {
            if (*astflags & rhai::ASTFlags::NEGATED) == rhai::ASTFlags::NEGATED {
                anyhow::bail!("Operator (?[]) not supported yet!");
            }
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes.push(ByteCode::Index);
        }
        Expr::And(binary_expr, _) => {
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            let pos = byte_codes.len();
            byte_codes.push(ByteCode::JumpIfFalse(0));
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes[pos] = ByteCode::JumpIfFalse(byte_codes.len() as OpSize);
        }
        Expr::Or(binary_expr, _) => {
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            let pos = byte_codes.len();
            byte_codes.push(ByteCode::JumpIfTrue(0));
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes[pos] = ByteCode::JumpIfTrue(byte_codes.len() as OpSize);
        }
        Expr::Coalesce(binary_expr, _) => {
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            let pos = byte_codes.len();
            byte_codes.push(ByteCode::JumpIfNotNull(0));
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes[pos] = ByteCode::JumpIfNotNull(byte_codes.len() as OpSize);
        }
        // Expr::Custom(..) => {
        //     anyhow::bail!("Custom syntax not supported yet!");
        // }
        _ => {
            anyhow::bail!("Unknown expression type for \"{:?}\"!", expr);
        }
    }
    return Ok(());
}

fn append_stmt(
    functions: &Vec<String>,
    variables: &mut Vec<String>,
    break_pos: &mut Vec<usize>,
    continue_pos: &mut Vec<usize>,
    byte_codes: &mut Vec<ByteCode>,
    stmt: &Stmt,
) -> anyhow::Result<()> {
    match stmt {
        Stmt::Noop(_) => {
            //Do nothing.
        }
        Stmt::If(flow_control, _) => {
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &flow_control.expr,
            )?;
            let jz_pos = byte_codes.len();
            byte_codes.push(ByteCode::JumpIfFalse(0));
            for sub_stmt in &flow_control.body {
                append_stmt(
                    functions,
                    variables,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_stmt,
                )?;
            }
            let jmp_pos = byte_codes.len();
            byte_codes.push(ByteCode::Jump(0));
            byte_codes[jz_pos] = ByteCode::JumpIfFalse(byte_codes.len() as OpSize);
            for sub_stmt in &flow_control.branch {
                append_stmt(
                    functions,
                    variables,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_stmt,
                )?;
            }
            byte_codes[jmp_pos] = ByteCode::Jump(byte_codes.len() as OpSize);
        }
        Stmt::Switch(..) => {
            anyhow::bail!("\"switch\" not supported yet!");
        }
        Stmt::While(flow_control, _) => {
            let start_pos = byte_codes.len();
            let jz_pos = match flow_control.expr {
                Expr::Unit(_) => usize::MAX,
                _ => {
                    append_expr(
                        functions,
                        variables,
                        break_pos,
                        continue_pos,
                        byte_codes,
                        &flow_control.expr,
                    )?;
                    byte_codes.push(ByteCode::JumpIfFalse(0));
                    byte_codes.len() - 1
                }
            };
            let mut new_break_pos = Vec::<usize>::new();
            let mut new_continue_pos = Vec::<usize>::new();
            for sub_stmt in &flow_control.body {
                append_stmt(
                    functions,
                    variables,
                    &mut new_break_pos,
                    &mut new_continue_pos,
                    byte_codes,
                    sub_stmt,
                )?;
            }
            byte_codes.push(ByteCode::Jump(start_pos as OpSize));
            let end_pos = byte_codes.len();
            for pos_break in &new_break_pos {
                byte_codes[*pos_break] = ByteCode::Jump(end_pos as OpSize);
            }
            for pos_continue in &new_continue_pos {
                byte_codes[*pos_continue] = ByteCode::Jump(start_pos as OpSize);
            }
            if jz_pos != usize::MAX {
                byte_codes[jz_pos] = ByteCode::JumpIfFalse(end_pos as OpSize);
            }
        }
        Stmt::Do(..) => {
            anyhow::bail!("\"do\" not supported yet!");
        }
        Stmt::For(_, _) => {
            anyhow::bail!("\"for\" not supported yet!");
        }
        Stmt::Var(data, _, _) => {
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &data.1,
            )?;
            let var_id = find_index_auto_append(variables, data.0.as_str());
            byte_codes.push(ByteCode::VarInit(var_id));
            byte_codes.push(ByteCode::PopStack);
        }
        Stmt::Assignment(data) => {
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &data.1.lhs,
            )?;
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                &data.1.rhs,
            )?;
            let op_str = match data.0.get_op_assignment_info() {
                Some(info) => info.3,
                None => "=",
            };
            let op_id = find_index(functions, op_str, "assignment operator")?;
            byte_codes.push(ByteCode::FnCall(op_id, 2));
            byte_codes.push(ByteCode::PopStack);
        }
        Stmt::FnCall(fn_call_expr, _) => {
            for sub_expr in &fn_call_expr.args {
                append_expr(
                    functions,
                    variables,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_expr,
                )?;
            }
            let fn_id = find_index(functions, fn_call_expr.name.as_str(), "function")?;
            byte_codes.push(ByteCode::FnCall(fn_id, fn_call_expr.args.len() as OpSize));
            byte_codes.push(ByteCode::PopStack);
        }
        Stmt::Block(stmt_block) => {
            for stmt in stmt_block.iter() {
                append_stmt(
                    functions,
                    variables,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    stmt,
                )?;
            }
        }
        Stmt::TryCatch(..) => {
            anyhow::bail!("\"try\" not supported yet!");
        }
        Stmt::Expr(expr) => {
            append_expr(
                functions,
                variables,
                break_pos,
                continue_pos,
                byte_codes,
                expr,
            )?;
        }
        Stmt::BreakLoop(_, astflags, _) => {
            if (*astflags & rhai::ASTFlags::BREAK) == rhai::ASTFlags::BREAK {
                //break
                break_pos.push(byte_codes.len());
            } else {
                //continue
                continue_pos.push(byte_codes.len());
            }
            byte_codes.push(ByteCode::Jump(0));
        }
        Stmt::Return(expr, astflags, _) => {
            if (*astflags & rhai::ASTFlags::BREAK) == rhai::ASTFlags::BREAK {
                anyhow::bail!("\"throw\" not supported yet!");
            } else {
                match expr {
                    Some(exp) => {
                        append_expr(
                            functions,
                            variables,
                            break_pos,
                            continue_pos,
                            byte_codes,
                            exp,
                        )?;
                    }
                    None => {
                        byte_codes.push(ByteCode::UnitConstant);
                    }
                }
                byte_codes.push(ByteCode::Return);
            }
        }
        // Stmt::Import(..) => todo!(),
        // Stmt::Export(..) => todo!(),
        // Stmt::Share(..) => todo!(),
        _ => {
            anyhow::bail!("Unknown statement type for \"{:?}\"!", stmt);
        }
    }
    return Ok(());
}

pub fn ast_to_byte_codes<T: DynamicValue>(
    executer: &Executer<T>,
    initial_variables: &mut Vec<String>,
    ast: &rhai::AST,
) -> anyhow::Result<Vec<ByteCode>> {
    let functions = executer.function_names();
    let mut byte_codes = Vec::<ByteCode>::new();
    let mut break_pos = Vec::<usize>::new();
    let mut continue_pos = Vec::<usize>::new();
    for stmt in ast.statements() {
        append_stmt(
            functions,
            initial_variables,
            &mut break_pos,
            &mut continue_pos,
            &mut byte_codes,
            stmt,
        )?;
    }
    if !break_pos.is_empty() || !continue_pos.is_empty() {
        anyhow::bail!("Invalid \"break\" or \"continue\" statements without a loop!");
    }
    return Ok(byte_codes);
}

pub fn script_to_byte_codes<T: DynamicValue>(
    executer: &Executer<T>,
    initial_variables: &mut Vec<String>,
    script: &str,
) -> anyhow::Result<Vec<ByteCode>> {
    let ast = COMPILE_ENGINE.compile(script)?;
    return ast_to_byte_codes(executer, initial_variables, &ast);
}

pub fn script_to_byte_codes_expression<T: DynamicValue>(
    executer: &Executer<T>,
    initial_variables: &mut Vec<String>,
    script: &str,
) -> anyhow::Result<Vec<ByteCode>> {
    let ast = COMPILE_ENGINE.compile_expression(script)?;
    return ast_to_byte_codes(executer, initial_variables, &ast);
}

pub fn run_byte_codes<T: DynamicValue>(
    executer: &Executer<T>,
    byte_codes: &Vec<ByteCode>,
    var_count: usize,
    init_vars: &Vec<T>,
) -> anyhow::Result<T> {
    let mut variables = vec![T::from_unit()?; var_count];
    for i in 0..usize::min(var_count, init_vars.len()) {
        variables[i] = init_vars[i].clone();
    }
    let mut variable_stack = Vec::<T>::new();
    let mut pos = 0usize;
    while pos < byte_codes.len() {
        //println!("{}: {:?}", pos, byte_codes[pos]);
        match &byte_codes[pos] {
            ByteCode::DynamicConstant(dynamic) => {
                variable_stack.push(T::from_dynamic(dynamic.clone())?);
            }
            ByteCode::UnitConstant => {
                variable_stack.push(T::from_unit()?);
            }
            ByteCode::BoolConstant(v) => {
                variable_stack.push(T::from_bool(*v)?);
            }
            ByteCode::IntegerConstant(v) => {
                variable_stack.push(T::from_integer(*v)?);
            }
            ByteCode::FloatConstant(v) => {
                variable_stack.push(T::from_float(*v)?);
            }
            ByteCode::CharConstant(v) => {
                variable_stack.push(T::from_char(*v)?);
            }
            ByteCode::StringConstant(v) => {
                variable_stack.push(T::from_string(v)?);
            }
            ByteCode::InterpolatedString(_) => {
                anyhow::bail!("InterpolatedString not supported yet!");
            }
            ByteCode::ConstructArray(_) => {
                anyhow::bail!("ConstructArray not supported yet!");
                // let mut arr = Vec::<rhai::Dynamic>::with_capacity(*l as usize);
                // if variable_stack.len() < *l as usize {
                //     anyhow::bail!("Not enough elements to construct array");
                // }
                // let start_position = variable_stack.len() - *l as usize;
                // for i in start_position..variable_stack.len() {
                //     arr.push(variable_stack[i].to_dynamic()?.clone());
                // }
                // variable_stack.truncate(start_position);
                // variable_stack.push(T::from_dynamic(rhai::Dynamic::from_array(arr))?);
            }
            ByteCode::Variable(var_id) => {
                variable_stack.push(T::from_variable_ref(*var_id)?);
            }
            ByteCode::FnCall(fn_index, fn_arg_count) => {
                let fn_arg_count_sz = *fn_arg_count as usize;
                if variable_stack.len() < fn_arg_count_sz {
                    anyhow::bail!("Not enough arguments for function call!");
                }
                let args = variable_stack.split_off(variable_stack.len() - fn_arg_count_sz);
                let res = executer.call_fn(*fn_index, args, &mut variables)?;
                variable_stack.push(res);
            }
            ByteCode::Jump(p) => {
                pos = *p as usize;
                continue;
            }
            ByteCode::JumpIfTrue(p) => match variable_stack.pop() {
                Some(val) => {
                    if val.to_bool(&variables)? {
                        pos = *p as usize;
                        continue;
                    }
                }
                None => {
                    anyhow::bail!("Not enough arguments for conditional jump!");
                }
            },
            ByteCode::JumpIfFalse(p) => match variable_stack.pop() {
                Some(val) => {
                    if !val.to_bool(&variables)? {
                        pos = *p as usize;
                        continue;
                    }
                }
                None => {
                    anyhow::bail!("Not enough arguments for conditional jump!");
                }
            },
            ByteCode::JumpIfNotNull(p) => match variable_stack.pop() {
                Some(val) => {
                    if !val.is_unit(&variables)? {
                        pos = *p as usize;
                        continue;
                    }
                }
                None => {
                    anyhow::bail!("Not enough arguments for conditional jump!");
                }
            },
            ByteCode::VarInit(var_id) => match variable_stack.last() {
                Some(val) => {
                    variables[*var_id as usize] = val.clone();
                }
                None => {
                    anyhow::bail!("Not enough arguments for variable declare!");
                }
            },
            ByteCode::Index => match variable_stack.pop() {
                Some(ind) => match variable_stack.last_mut() {
                    Some(r) => {
                        r.ref_append_index(ind.to_size(&variables)?)?;
                    }
                    None => {
                        anyhow::bail!("Not enough arguments for index!");
                    }
                },
                None => {
                    anyhow::bail!("Not enough arguments for index!");
                }
            },
            ByteCode::Return => match variable_stack.pop() {
                Some(value) => {
                    return Ok(value);
                }
                None => {
                    anyhow::bail!("Missing return value!");
                }
            },
            ByteCode::PopStack => {
                variable_stack.pop();
            }
        }
        pos += 1;
    }
    //println!("Stack size: {}",variable_stack.len());
    match variable_stack.pop() {
        Some(value) => return Ok(value),
        None => {
            return T::from_unit();
        }
    }
}
