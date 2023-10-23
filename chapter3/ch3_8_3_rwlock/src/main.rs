use std::sync::RwLock;

fn main() {
    let lock = RwLock::new(10);
    {
        // immutableな参照を取得
        // Read Lockは何度でも行える。
        let v1 = lock.read().unwrap();
        let v2 = lock.read().unwrap();
        println!("v1 = {}", v1);
        println!("v2 = {}", v2);
    }

    {
        // mutableな参照を取得
        // Write Lockはスコープ内で1つのみ
        let mut v = lock.write().unwrap();
        *v = 7;
        println!("v = {}", v);
    }
}
