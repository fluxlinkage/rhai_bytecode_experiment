mod sample;

use std::cell::RefCell;
use std::rc::Rc;
use rhai_bytecode::DynamicValue;
use sample::SimpleDynamicValue;

fn new_array_for_rhai(l:rhai_bytecode::INT,v:rhai_bytecode::rhai::Dynamic)->rhai_bytecode::rhai::Dynamic{
    return rhai_bytecode::rhai::Dynamic::from_array(vec![v; l as usize]);
}

fn new_array_for_rhai_bytecode(args: &[Rc<RefCell<SimpleDynamicValue>>]) -> anyhow::Result<Rc<RefCell<SimpleDynamicValue>>>  {
    let l=args[0].borrow().to_size()? as usize; // Never panics when single-threaded.
    let element = args[1].borrow().clone(); // Never panics when single-threaded.
    let mut new_ary=rhai_bytecode::VEC::with_capacity(l);
    for _i in 0..l{
        new_ary.push(Rc::new(RefCell::new(element.clone())));
    }
    return Ok(Rc::new(RefCell::new(SimpleDynamicValue::Array(new_ary))));
}

fn main() {
    const ROUNDS: usize = 3;
    let script = "//! This script uses the Sieve of Eratosthenes to calculate prime numbers.
const MAX_NUMBER_TO_CHECK = 1_000_000;
let prime_mask = new_array(MAX_NUMBER_TO_CHECK + 1, true);
prime_mask[0] = false;
prime_mask[1] = false;
let total_primes_found = 0;
for p in 2..=MAX_NUMBER_TO_CHECK {
    if !prime_mask[p] { continue; }
    total_primes_found += 1;
    let i = 2 * p;
    while i <= MAX_NUMBER_TO_CHECK {
        prime_mask[i] = false;
        i += p;
    }
}
total_primes_found";
    let mut engine = rhai_bytecode::rhai::Engine::new();
    engine.register_fn("new_array", new_array_for_rhai);
    let ast = engine.compile(script).unwrap();
    let mut executer = sample::new_executer().unwrap();
    executer.add_fn("new_array", new_array_for_rhai_bytecode,2,2).unwrap();
    let mut variable_names = Vec::<String>::new();
    let byte_codes= rhai_bytecode::ast_to_byte_codes(&executer, &mut variable_names, &ast).unwrap();
    let json = serde_json::to_string(&byte_codes).unwrap();
    println!("Serilized JSON = {}", json);
    println!("JSON length = {} ({}% of original script)", json.len(),json.len()*100/script.len());
    let byte_codes_restored = serde_json::from_str::<Vec<rhai_bytecode::ByteCode>>(&json).unwrap();
    let mut times_byte_code = Vec::<f64>::new();
    let mut times_ast = Vec::<f64>::new();
    println!("Round\tResults\t\tTime");
    println!("\tBytecode\tAST\tBytecode\tAST");
    for r in 0..ROUNDS {
        let now = std::time::Instant::now();
        let res_byte_code = rhai_bytecode::run_byte_codes::<SimpleDynamicValue>(
            &executer,
            &byte_codes_restored,
            &vec![],
        )
        .unwrap();
        let time_byte_code=now.elapsed().as_secs_f64();
        let now = std::time::Instant::now();
        let res_ast = engine
            .eval_ast::<rhai_bytecode::rhai::Dynamic>(&ast)
            .unwrap();
        let time_ast=now.elapsed().as_secs_f64();
        println!("{}\t{:?}\t{:?}\t{}\t{}",r, res_byte_code,res_ast,time_byte_code,time_ast);
        times_byte_code.push(time_byte_code);
        times_ast.push(time_ast);
        // Results should be 78498.
    }
    times_byte_code.sort_by(|a, b| a.partial_cmp(b).unwrap());
    times_ast.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("Median time:");
    println!("Bytecode: {} ({}%)",times_byte_code[ROUNDS / 2],((times_byte_code[ROUNDS / 2] * 100.0) / times_ast[ROUNDS / 2]+0.5) as u16);
    println!("AST: {} (100%)",times_ast[ROUNDS / 2]);
}
