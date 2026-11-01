//! `jobs` threads over a list: each takes the next item until none is left.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// `run(i, &items[i])` for every item on `jobs` threads; the results in item order.
pub fn run_all<T: Sync, R: Send>(items: &[T], jobs: usize, run: impl Fn(usize, &T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<Option<R>>> = Mutex::new((0..items.len()).map(|_| None).collect());
    std::thread::scope(|scope| {
        for _ in 0..jobs.max(1) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= items.len() {
                    break;
                }
                let r = run(i, &items[i]);
                results.lock().unwrap_or_else(|e| e.into_inner())[i] = Some(r);
            });
        }
    });
    results.into_inner().unwrap_or_else(|e| e.into_inner()).into_iter().map(|r| r.expect("every item ran")).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn results_keep_the_item_order() {
        let items: Vec<u32> = (0..50).collect();
        assert_eq!(super::run_all(&items, 4, |i, &x| (i as u32, x * 2)), items.iter().map(|&x| (x, x * 2)).collect::<Vec<_>>());
    }
}
