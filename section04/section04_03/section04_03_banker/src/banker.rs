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
}