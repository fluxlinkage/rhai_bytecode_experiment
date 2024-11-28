pub use rhai;
use rhai::{Expr, Stmt};

thread_local! {
    static COMPILE_ENGINE: std::cell::RefCell<rhai::Engine> = std::cell::RefCell::new(rhai::Engine::new_raw());
}

#[cfg(feature = "size16")]
pub type OpSize = u16;
#[cfg(feature = "size32")]
pub type OpSize = u32;
#[cfg(feature = "size64")]
pub type OpSize = u64;

pub type INT = rhai::INT;
pub type FLOAT = rhai::FLOAT;

#[derive(serde::Serialize, serde::Deserialize)]
pub enum ByteCode {
    /// DynamicConstant
    DC(rhai::Dynamic),
    /// UnitConstant
    UC,
    /// BoolConstant
    BC(bool),
    /// IntegerConstant
    IC(INT),
    /// FloatConstant
    FC(FLOAT),
    /// CharConstant
    CC(char),
    /// StringConstant
    SC(String),
    /// InterpolatedString
    IS(OpSize),
    /// ConstructArray
    CA(OpSize),
    /// Variable
    V(OpSize),
    /// FnCall
    F(OpSize, OpSize),
    /// Jump
    J(OpSize),
    /// JumpIfTrue
    JT(OpSize),
    /// JumpIfFalse
    JF(OpSize),
    /// JumpIfNotNull
    JNN(OpSize),
    /// VarInit
    VI(OpSize),
    /// Index
    I,
    /// Return
    R,
    /// PopStack
    P,
}

impl std::fmt::Debug for ByteCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DC(arg0) => f.debug_tuple("DynamicConstant").field(arg0).finish(),
            Self::UC => write!(f, "UnitConstant"),
            Self::BC(arg0) => f.debug_tuple("BoolConstant").field(arg0).finish(),
            Self::IC(arg0) => f.debug_tuple("IntegerConstant").field(arg0).finish(),
            Self::FC(arg0) => f.debug_tuple("FloatConstant").field(arg0).finish(),
            Self::CC(arg0) => f.debug_tuple("CharConstant").field(arg0).finish(),
            Self::SC(arg0) => f.debug_tuple("StringConstant").field(arg0).finish(),
            Self::IS(arg0) => f.debug_tuple("InterpolatedString").field(arg0).finish(),
            Self::CA(arg0) => f.debug_tuple("ConstructArray").field(arg0).finish(),
            Self::V(arg0) => f.debug_tuple("Variable").field(arg0).finish(),
            Self::F(arg0, arg1) => f.debug_tuple("FnCall").field(arg0).field(arg1).finish(),
            Self::J(arg0) => f.debug_tuple("Jump").field(arg0).finish(),
            Self::JT(arg0) => f.debug_tuple("JumpIfTrue").field(arg0).finish(),
            Self::JF(arg0) => f.debug_tuple("JumpIfFalse").field(arg0).finish(),
            Self::JNN(arg0) => f.debug_tuple("JumpIfNotNull").field(arg0).finish(),
            Self::VI(arg0) => f.debug_tuple("VarInit").field(arg0).finish(),
            Self::I => write!(f, "Index"),
            Self::R => write!(f, "Return"),
            Self::P => write!(f, "PopStack"),
        }
    }
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
    fn from_variable_ref(var:std::rc::Rc<std::cell::RefCell<Self>>) -> anyhow::Result<Self>;
    fn is_unit(&self) -> anyhow::Result<bool>;
    fn to_bool(&self) -> anyhow::Result<bool>;
    fn to_size(&self) -> anyhow::Result<OpSize>;
    fn get_value(&self) -> anyhow::Result<Self>;
    fn set_value(&self,val:Self) -> anyhow::Result<()>;
    fn enter_index(&mut self, ind: OpSize) -> anyhow::Result<()>;
}

pub struct Executer<T: DynamicValue> {
    fn_names: Vec<String>,
    fns: Vec<Box<dyn Fn(&Vec<T>) -> anyhow::Result<T>>>,
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
    pub fn add_fn<F:Fn(&Vec<T>) -> anyhow::Result<T>+'static>(
        &mut self,
        name: impl ToString,
        func: F,
    ) -> anyhow::Result<()> {
        let name_string = name.to_string();
        if self.fn_names.contains(&name_string) {
            anyhow::bail!("Function \"{}\" already exists!", name_string);
        } else {
            self.fns.push(Box::new(func));
            self.fn_names.push(name_string);
            return Ok(());
        }
    }
    fn call_fn(&self, index: OpSize, args: &Vec<T>) -> anyhow::Result<T> {
        let ind = index as usize;
        if ind >= self.fns.len() {
            anyhow::bail!("Function #{} does not exist!", index);
        } else {
            return self.fns[ind](args);
        }
    }
}

fn find_index(vec: &Vec<String>, name: &str, type_str: &str) -> anyhow::Result<OpSize> {
    match vec.iter().rposition(|x| x == name) {
        Some(i) => {
            return Ok(i as OpSize);
        }
        None => {
            anyhow::bail!("Undefined {} \"{}\"!", type_str, name);
        }
    }
}

fn append_return_index(vec: &mut Vec<String>, name: &str) -> OpSize {
    vec.push(name.to_string());
    return (vec.len() - 1) as OpSize;
    // match vec.iter().position(|x| x == name) {
    //     Some(i) => {
    //         return i as OpSize;
    //     }
    //     None => {
    //         vec.push(name.to_string());
    //         return (vec.len() - 1) as OpSize;
    //     }
    // }
}

fn append_expr(
    functions: &Vec<String>,
    variables: &mut Vec<String>,
    max_variable_count: &mut OpSize,
    break_pos: &mut Vec<usize>,
    continue_pos: &mut Vec<usize>,
    byte_codes: &mut Vec<ByteCode>,
    expr: &Expr,
) -> anyhow::Result<()> {
    match expr {
        Expr::DynamicConstant(dynamic, _) => {
            byte_codes.push(ByteCode::DC(*dynamic.clone()));
        }
        Expr::BoolConstant(v, _) => {
            byte_codes.push(ByteCode::BC(*v));
        }
        Expr::IntegerConstant(v, _) => {
            byte_codes.push(ByteCode::IC(*v));
        }
        Expr::FloatConstant(float_wrapper, _) => {
            byte_codes.push(ByteCode::FC(*float_wrapper.as_ref()));
        }
        Expr::CharConstant(v, _) => {
            byte_codes.push(ByteCode::CC(*v));
        }
        Expr::StringConstant(immutable_string, _) => {
            byte_codes.push(ByteCode::SC(immutable_string.to_string()));
        }
        Expr::InterpolatedString(thin_vec, _) => {
            for expr in thin_vec {
                append_expr(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    expr,
                )?;
            }
            byte_codes.push(ByteCode::IS(thin_vec.len() as OpSize));
        }
        Expr::Array(thin_vec, _) => {
            for sub_expr in thin_vec {
                append_expr(
                    functions,
                    variables,
                    max_variable_count,
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
            byte_codes.push(ByteCode::UC);
        }
        Expr::Variable(data, _, _) => {
            let var_id = find_index(variables, data.1.as_str(), "variable")?;
            byte_codes.push(ByteCode::V(var_id));
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
                    max_variable_count,
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
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_expr,
                )?;
            }
            let fn_id = find_index(functions, fn_call_expr.name.as_str(), "function")?;
            byte_codes.push(ByteCode::F(fn_id, fn_call_expr.args.len() as OpSize));
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
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes.push(ByteCode::I);
        }
        Expr::And(binary_expr, _) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            let pos = byte_codes.len();
            byte_codes.push(ByteCode::JF(0));
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes[pos] = ByteCode::JF(byte_codes.len() as OpSize);
        }
        Expr::Or(binary_expr, _) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            let pos = byte_codes.len();
            byte_codes.push(ByteCode::JT(0));
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes[pos] = ByteCode::JT(byte_codes.len() as OpSize);
        }
        Expr::Coalesce(binary_expr, _) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.lhs,
            )?;
            let pos = byte_codes.len();
            byte_codes.push(ByteCode::JNN(0));
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &binary_expr.rhs,
            )?;
            byte_codes[pos] = ByteCode::JNN(byte_codes.len() as OpSize);
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
    max_variable_count: &mut OpSize,
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
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &flow_control.expr,
            )?;
            let jz_pos = byte_codes.len();
            byte_codes.push(ByteCode::JF(0));
            let var_len=variables.len();
            for sub_stmt in &flow_control.body {
                append_stmt(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_stmt,
                )?;
            }
            variables.truncate(var_len);
            let jmp_pos = byte_codes.len();
            byte_codes.push(ByteCode::J(0));
            byte_codes[jz_pos] = ByteCode::JF(byte_codes.len() as OpSize);
            let var_len=variables.len();
            for sub_stmt in &flow_control.branch {
                append_stmt(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_stmt,
                )?;
            }
            variables.truncate(var_len);
            byte_codes[jmp_pos] = ByteCode::J(byte_codes.len() as OpSize);
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
                        max_variable_count,
                        break_pos,
                        continue_pos,
                        byte_codes,
                        &flow_control.expr,
                    )?;
                    byte_codes.push(ByteCode::JF(0));
                    byte_codes.len() - 1
                }
            };
            let mut new_break_pos = Vec::<usize>::new();
            let mut new_continue_pos = Vec::<usize>::new();
            let var_len=variables.len();
            for sub_stmt in &flow_control.body {
                append_stmt(
                    functions,
                    variables,
                    max_variable_count,
                    &mut new_break_pos,
                    &mut new_continue_pos,
                    byte_codes,
                    sub_stmt,
                )?;
            }
            variables.truncate(var_len);
            byte_codes.push(ByteCode::J(start_pos as OpSize));
            let end_pos = byte_codes.len();
            for pos_break in &new_break_pos {
                byte_codes[*pos_break] = ByteCode::J(end_pos as OpSize);
            }
            for pos_continue in &new_continue_pos {
                byte_codes[*pos_continue] = ByteCode::J(start_pos as OpSize);
            }
            if jz_pos != usize::MAX {
                byte_codes[jz_pos] = ByteCode::JF(end_pos as OpSize);
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
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &data.1,
            )?;
            let var_id = append_return_index(variables, data.0.as_str());
            if var_id+1>*max_variable_count {
                *max_variable_count=var_id+1;
            }
            byte_codes.push(ByteCode::VI(var_id));
            byte_codes.push(ByteCode::P);
        }
        Stmt::Assignment(data) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
                break_pos,
                continue_pos,
                byte_codes,
                &data.1.lhs,
            )?;
            append_expr(
                functions,
                variables,
                max_variable_count,
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
            byte_codes.push(ByteCode::F(op_id, 2));
            byte_codes.push(ByteCode::P);
        }
        Stmt::FnCall(fn_call_expr, _) => {
            for sub_expr in &fn_call_expr.args {
                append_expr(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    sub_expr,
                )?;
            }
            let fn_id = find_index(functions, fn_call_expr.name.as_str(), "function")?;
            byte_codes.push(ByteCode::F(fn_id, fn_call_expr.args.len() as OpSize));
            byte_codes.push(ByteCode::P);
        }
        Stmt::Block(stmt_block) => {
            let var_len=variables.len();
            for stmt in stmt_block.iter() {
                append_stmt(
                    functions,
                    variables,
                    max_variable_count,
                    break_pos,
                    continue_pos,
                    byte_codes,
                    stmt,
                )?;
            }
            variables.truncate(var_len);
        }
        Stmt::TryCatch(..) => {
            anyhow::bail!("\"try\" not supported yet!");
        }
        Stmt::Expr(expr) => {
            append_expr(
                functions,
                variables,
                max_variable_count,
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
            byte_codes.push(ByteCode::J(0));
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
                            max_variable_count,
                            break_pos,
                            continue_pos,
                            byte_codes,
                            exp,
                        )?;
                    }
                    None => {
                        byte_codes.push(ByteCode::UC);
                    }
                }
                byte_codes.push(ByteCode::R);
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
) -> anyhow::Result<(Vec<ByteCode>,OpSize)> {
    let functions = executer.function_names();
    let mut byte_codes = Vec::<ByteCode>::new();
    let mut break_pos = Vec::<usize>::new();
    let mut continue_pos = Vec::<usize>::new();
    let mut max_variable_count=initial_variables.len() as OpSize;
    for stmt in ast.statements() {
        append_stmt(
            functions,
            initial_variables,
            &mut max_variable_count,
            &mut break_pos,
            &mut continue_pos,
            &mut byte_codes,
            stmt,
        )?;
    }
    if !break_pos.is_empty() || !continue_pos.is_empty() {
        anyhow::bail!("Invalid \"break\" or \"continue\" statements without a loop!");
    }
    match byte_codes.last() {
        Some(code) => match code {
            ByteCode::P => {
                byte_codes.pop();
            }
            _ => {}
        },
        None => {}
    }
    return Ok((byte_codes,max_variable_count));
}

pub fn script_to_byte_codes<T: DynamicValue>(
    executer: &Executer<T>,
    initial_variables: &mut Vec<String>,
    script: &str,
) -> anyhow::Result<(Vec<ByteCode>,OpSize)> {
    let ast=COMPILE_ENGINE.with_borrow(|engine|engine.compile(script))?;
    return ast_to_byte_codes(executer, initial_variables, &ast);
}

pub fn script_to_byte_codes_expression<T: DynamicValue>(
    executer: &Executer<T>,
    initial_variables: &mut Vec<String>,
    script: &str,
) -> anyhow::Result<(Vec<ByteCode>,OpSize)> {
    let ast=COMPILE_ENGINE.with_borrow(|engine|engine.compile_expression(script))?;
    return ast_to_byte_codes(executer, initial_variables, &ast);
}

pub fn script_to_byte_codes_expression_no_new_variables<T: DynamicValue>(
    executer: &Executer<T>,
    initial_variables: &mut Vec<String>,
    script: &str,
) -> anyhow::Result<(Vec<ByteCode>,OpSize)> {
    let ast=COMPILE_ENGINE.with_borrow(|engine|engine.compile_expression(script))?;
    let init_len = initial_variables.len();
    let res = ast_to_byte_codes(executer, initial_variables, &ast)?;
    if initial_variables.len() != init_len {
        initial_variables.truncate(init_len);
        anyhow::bail!("The script should not declare new variables!");
    } else {
        return Ok(res);
    }
}

pub fn run_byte_codes<T: DynamicValue>(
    executer: &Executer<T>,
    byte_codes: &Vec<ByteCode>,
    var_count: usize,
    init_vars: &Vec<T>,
) -> anyhow::Result<T> {
    let mut variables=Vec::<std::rc::Rc<std::cell::RefCell<T>>>::with_capacity(var_count);
    let init_len=usize::min(var_count, init_vars.len());
    for i in 0..init_len {
        variables.push(std::rc::Rc::new(std::cell::RefCell::<T>::new(init_vars[i].clone())));
    }
    for _i in init_len..var_count {
        variables.push(std::rc::Rc::new(std::cell::RefCell::<T>::new(T::from_unit()?)));
    }
    let mut variable_stack = Vec::<T>::new();
    let mut pos = 0usize;
    while pos < byte_codes.len() {
        //println!("{}: {:?}", pos, byte_codes[pos]);
        match &byte_codes[pos] {
            ByteCode::DC(dynamic) => {
                variable_stack.push(T::from_dynamic(dynamic.clone())?);
            }
            ByteCode::UC => {
                variable_stack.push(T::from_unit()?);
            }
            ByteCode::BC(v) => {
                variable_stack.push(T::from_bool(*v)?);
            }
            ByteCode::IC(v) => {
                variable_stack.push(T::from_integer(*v)?);
            }
            ByteCode::FC(v) => {
                variable_stack.push(T::from_float(*v)?);
            }
            ByteCode::CC(v) => {
                variable_stack.push(T::from_char(*v)?);
            }
            ByteCode::SC(v) => {
                variable_stack.push(T::from_string(v)?);
            }
            ByteCode::IS(_) => {
                anyhow::bail!("InterpolatedString not supported yet!");
            }
            ByteCode::CA(_) => {
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
            ByteCode::V(var_id) => {
                variable_stack.push(T::from_variable_ref(variables[*var_id as usize].clone())?);
            }
            ByteCode::F(fn_index, fn_arg_count) => {
                let fn_arg_count_sz = *fn_arg_count as usize;
                if variable_stack.len() < fn_arg_count_sz {
                    anyhow::bail!("Not enough arguments for function call!");
                }
                let args = variable_stack.split_off(variable_stack.len() - fn_arg_count_sz);
                let res = executer.call_fn(*fn_index, &args)?;
                variable_stack.push(res);
            }
            ByteCode::J(p) => {
                pos = *p as usize;
                continue;
            }
            ByteCode::JT(p) => match variable_stack.pop() {
                Some(val) => {
                    if val.to_bool()? {
                        pos = *p as usize;
                        continue;
                    }
                }
                None => {
                    anyhow::bail!("Not enough arguments for conditional jump!");
                }
            },
            ByteCode::JF(p) => match variable_stack.pop() {
                Some(val) => {
                    if !val.to_bool()? {
                        pos = *p as usize;
                        continue;
                    }
                }
                None => {
                    anyhow::bail!("Not enough arguments for conditional jump!");
                }
            },
            ByteCode::JNN(p) => match variable_stack.pop() {
                Some(val) => {
                    if !val.is_unit()? {
                        pos = *p as usize;
                        continue;
                    }
                }
                None => {
                    anyhow::bail!("Not enough arguments for conditional jump!");
                }
            },
            ByteCode::VI(var_id) => match variable_stack.last() {
                Some(val) => {
                    *(variables[*var_id as usize].try_borrow_mut()?)=val.get_value()?.clone();
                }
                None => {
                    anyhow::bail!("Not enough arguments for variable declare!");
                }
            },
            ByteCode::I => match variable_stack.pop() {
                Some(ind) => match variable_stack.last_mut() {
                    Some(r) => {
                        r.enter_index(ind.to_size()?)?;
                    }
                    None => {
                        anyhow::bail!("Not enough arguments for index!");
                    }
                },
                None => {
                    anyhow::bail!("Not enough arguments for index!");
                }
            },
            ByteCode::R => match variable_stack.pop() {
                Some(value) => {
                    return Ok(value);
                }
                None => {
                    anyhow::bail!("Missing return value!");
                }
            },
            ByteCode::P => {
                variable_stack.pop();
            }
        }
        pos += 1;
    }
    //println!("Stack size: {}",variable_stack.len());
    match variable_stack.pop() {
        Some(value) => return value.get_value(),
        None => {
            return T::from_unit();
        }
    }
}
