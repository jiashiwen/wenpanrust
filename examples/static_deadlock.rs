use std::collections::HashMap;
use anyhow::anyhow;
use std::sync::RwLock;

lazy_static::lazy_static! {
    static ref GLOBAL_MAP: RwLock<HashMap<String,String>> = RwLock::new({
        let map = HashMap::new();
        map
    });
}

fn main() {
    for i in 0..3 {
        insert_global_map(i.to_string(), i.to_string())
    }
    print_global_map();
    get_and_remove(1);
    println!("finished!");
}

fn insert_global_map(k: String, v: String) {
    let mut gpw = GLOBAL_MAP.write().unwrap();
    gpw.insert(k, v);
}

fn print_global_map() {
    let gpr = GLOBAL_MAP.read().unwrap();
    for pair in gpr.iter() {
        println!("{:?}", pair);
    }
}

fn get_and_remove_deadlock(k: i32) {
    println!("execute get_and_remove");
    let gpr = GLOBAL_MAP.read().unwrap();
    let v = gpr.get(&*k.to_string().clone());
    let mut gpw = GLOBAL_MAP.write().unwrap();
    gpw.remove(&*k.to_string().clone());
}

fn get_and_remove(k: i32) {
    let v = {
        let gpr = GLOBAL_MAP.read().unwrap();
        let v = gpr.get(&*k.to_string().clone());
        match v {
            None => Err(anyhow!("")),
            Some(pair) => Ok(pair.to_string().clone()),
        }
    };
    let vstr = v.unwrap();
    println!("get value is {:?}", vstr.clone());
    let mut gpw = GLOBAL_MAP.write().unwrap();
    gpw.remove(&*vstr);
}