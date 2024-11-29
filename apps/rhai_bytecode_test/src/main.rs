mod sample;

use rhai_bytecode::DynamicValue;
use sample::SimpleDynamicValue;

fn new_array_for_rhai(l:rhai_bytecode::INT,v:rhai_bytecode::rhai::Dynamic)->rhai_bytecode::rhai::Dynamic{
    return rhai_bytecode::rhai::Dynamic::from_array(vec![v; l as usize]);
}

fn new_array_for_rhai_bytecode(args: &Vec<SimpleDynamicValue>) -> anyhow::Result<SimpleDynamicValue>{
    if args.len() != 2 {
        anyhow::bail!(
            "Function \"new_array\" needs 2 arguments, but {} provided!",
            args.len()
        );
    }else{
        let l = args[0].get_value()?.to_size()? as usize;
        let v = args[1].get_value()?;
        let mut vec=rhai_bytecode::VEC::<std::rc::Rc<std::cell::RefCell<SimpleDynamicValue>>>::with_capacity(l);
        for _i in 0..l {
            vec.push(std::rc::Rc::new(std::cell::RefCell::new(v.clone())));
        }
        return Ok(SimpleDynamicValue::Array(vec));
    }
}

fn main() {
    // let a = std::thread::spawn(|| {
    //     let b=rhai_bytecode::ByteCode::UC;
    //     b
    // });
    // let script = "let ary = [1,2,3];
    // ary[1]";
    // let script = "let x = 10_000_000;
    // while x > 0 {
    //     x -= 1;
    // }";
    let script = "//! This script uses the Sieve of Eratosthenes to calculate prime numbers.
    const MAX_NUMBER_TO_CHECK = 1_000_000;
    let prime_mask = new_array(MAX_NUMBER_TO_CHECK + 1, true);
    prime_mask[0] = false;
    prime_mask[1] = false;
    let total_primes_found = 0;
    let p = 1;
    while p < MAX_NUMBER_TO_CHECK {
        p += 1;
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
    executer.add_fn("new_array", new_array_for_rhai_bytecode).unwrap();
    let mut variable_names = Vec::<String>::new();
    let byte_codes= rhai_bytecode::ast_to_byte_codes(&executer, &mut variable_names, &ast).unwrap();
    let json = serde_json::to_string(&byte_codes).unwrap();
    println!("Serilized JSON = {}", json);
    println!("JSON length = {} ({}% of original script)", json.len(),json.len()*100/script.len());
    let byte_codes_restored = serde_json::from_str::<Vec<rhai_bytecode::ByteCode>>(&json).unwrap();
    let now = std::time::Instant::now();
    let res_byte_code = rhai_bytecode::run_byte_codes(
        &executer,
        &byte_codes_restored,
        &vec![],
    )
    .unwrap();
    let time_byte_code=now.elapsed().as_secs_f64();
    println!(
        "Finished (Bytecode). Run time = {} seconds.",
        time_byte_code
    );
    println!("Result (Bytecode) = {:?}", res_byte_code);
    let now = std::time::Instant::now();
    let res_ast = engine
        .eval_ast::<rhai_bytecode::rhai::Dynamic>(&ast)
        .unwrap();
    let time_ast=now.elapsed().as_secs_f64();
    println!(
        "Finished (AST). Run time = {} seconds.",
        time_ast
    );
    println!("Result (AST) = {:?}", res_ast);
    // Should be 78498.
    println!("Bytecode timecost = {}% of AST", (time_byte_code*100.0/time_ast+0.5) as i32);
}
