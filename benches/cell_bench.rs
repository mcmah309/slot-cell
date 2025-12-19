use criterion::{Criterion, black_box, criterion_group, criterion_main};
use slot_cell::SlotCell;
use std::cell::RefCell;

fn bench_i32_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("i32_Update");

    group.bench_function("RefCell::borrow_mut", |b| {
        let cell = RefCell::new(0i32);
        b.iter(|| {
            let cell = black_box(&cell);
            let mut borrow = cell.borrow_mut();
            *borrow += 1;
            black_box(borrow);
        })
    });

    group.bench_function("SlotCell::take_put", |b| {
        let cell = SlotCell::new(0i32);
        b.iter(|| {
            let cell = black_box(&cell);
            let mut val = cell.take();
            val += 1;
            cell.put(black_box(val));
        })
    });

    group.finish();
}

fn bench_string_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("String_Update");

    group.bench_function("RefCell::borrow_mut", |b| {
        let cell = RefCell::new(String::from("hello"));
        b.iter(|| {
            let cell = black_box(&cell);
            let mut borrow = cell.borrow_mut();
            borrow.push('!');
            if borrow.len() > 100 {
                borrow.clear();
            }
            black_box(borrow);
        })
    });

    group.bench_function("SlotCell::take_put", |b| {
        let cell = SlotCell::new(String::from("hello"));
        b.iter(|| {
            let cell = black_box(&cell);
            let mut val = cell.take();
            val.push('!');
            if val.len() > 100 {
                val.clear();
            }
            cell.put(black_box(val));
        })
    });

    group.finish();
}

fn bench_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("Access_Overhead");

    group.bench_function("RefCell_overhead", |b| {
        let cell = RefCell::new(100);
        b.iter(|| {
            let cell = black_box(&cell);
            let val = black_box(cell.borrow_mut());
            black_box(val);
        })
    });

    group.bench_function("SlotCell_overhead", |b| {
        let cell = SlotCell::new(100);
        b.iter(|| {
            let cell = black_box(&cell);
            let val = black_box(cell.take());
            cell.put(black_box(val));
        })
    });

    group.finish();
}

criterion_group!(benches, bench_i32_update, bench_string_update, bench_overhead);
criterion_main!(benches);
