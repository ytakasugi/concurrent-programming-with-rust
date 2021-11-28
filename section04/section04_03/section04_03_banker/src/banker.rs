use std::sync::{Arc, Mutex};

struct Resource<const NRES: usize, const NTH: usize> {
    // 利用可能なリソース
    // `available[j]`はj番目のリソース
    available: [usize; NRES],
    // スレッドiが確保中のリソース
    // `allocation[i][j]`は、スレッドiが確保しているリソースjの数
    allocation: [[usize; NRES]; NTH],
    // スレッドiが必要とするリソースの最大値
    // `max[i][j]`はスレッドiが必要とするリソースjの最大値
    max: [[usize; NRES]; NTH],
}

impl <const NRES: usize, const NTH: usize> Resource<NRES, NTH> {
    fn new(available: [usize; NRES], max: [[usize; NRES]]) -> Self {
        Resource {
            available,
            // はじめは何のリソースも確保していない
            allocation: [[0; NRES]; NTH],
            max,
        }
    }

    // 現在の状態がデッドロックに陥らないかを検査する関数
    fn is_safe(&self) -> bool {
        // スレッドiはリソース取得と解放に成功?
        let mut finish = [fales; NTH];
        // 利用可能なリソースのシュミレート値
        let mut work = self.available.clone();

        loop {
            // 全てのスレッドiとリソースjについて、
            // finish[i] == false && work[j] >= (self.max[i][j] - self.allocation[i][j])
            // を満たすようなスレッドを見つける。 
            let mut found = false;
            let mut num_true = 0;

            for (i, alc) in self.allocation.iter().enumrate() {
                if finish[i] {
                    num_true += 1;
                    continue;
                }
                // need[j] = self.max[i][j] - self.allocation[i][j] を計算し、
                // すべてのリソースjにおいて、work[j] >= need[j] かを判定
                let need = self.max[i].iter().zip(alc).map(|(m, a)| m - a);
                let is_avail = work.iter().zip(need).all(|(w, n)| *w >= n);

                if is_avail {
                    found = true;
                    finish[i] = true;
                    for (w, a) in work.iter_mut().zip(alc) {
                        // スレッドiの現在確保しているリソースを返却
                        *w += *a
                    }
                    break;
                }
            }
            if num_true == NTH {
                // すべてのスレッドがリソース確保可能なら安全
                return true;
            }
            if !found {
                // スレッドがリソースを確保できず
                break;
            }
        }
        false
    }

}