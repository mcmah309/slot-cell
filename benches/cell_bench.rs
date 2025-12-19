use criterion::{Criterion, black_box, criterion_group, criterion_main};
use slot_cell::SlotCell;
use std::cell::RefCell;

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

fn bench_i32_update(c: &mut Criterion) {
    let mut group = c.benchmark_group("i32_Update");

    group.bench_function("RefCell::borrow_mut", |b| {
        let cell = RefCell::new(0i32);
        b.iter(|| {
            let cell = black_box(&cell);
            let mut borrow = black_box(cell.borrow_mut());
            *borrow += 1;
            black_box(borrow);
        })
    });

    group.bench_function("SlotCell::take_put", |b| {
        let cell = SlotCell::new(0i32);
        b.iter(|| {
            let cell = black_box(&cell);
            let mut val = black_box(cell.take());
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
            let mut borrow = black_box(cell.borrow_mut());
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
            let mut val = black_box(cell.take());
            val.push('!');
            if val.len() > 100 {
                val.clear();
            }
            cell.put(black_box(val));
        })
    });

    group.finish();
}

// A large struct representing some game entity or complex state
// Total size: ~1KB
struct GameEntity {
    position: [f64; 3],
    velocity: [f64; 3],
    rotation: [f64; 4],
    health: f64,
    inventory: [u32; 64],
    status_effects: [u8; 128],
    animation_state: [f32; 100],
    metadata: [u64; 20],
}

impl GameEntity {
    fn new() -> Self {
        Self {
            position: [0.0; 3],
            velocity: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            health: 100.0,
            inventory: [0; 64],
            status_effects: [0; 128],
            animation_state: [0.0; 100],
            metadata: [0; 20],
        }
    }

    fn update(&mut self) {
        self.position[0] += self.velocity[0] * 0.016;
        self.position[1] += self.velocity[1] * 0.016;
        self.position[2] += self.velocity[2] * 0.016;
        self.health = (self.health - 0.1).max(0.0);
        self.animation_state[0] += 1.0;
    }
}

fn bench_large_struct(c: &mut Criterion) {
    let mut group = c.benchmark_group("GameEntity_Update");

    group.bench_function("RefCell::borrow_mut", |b| {
        let cell = RefCell::new(GameEntity::new());
        b.iter(|| {
            let cell = black_box(&cell);
            let mut borrow = black_box(cell.borrow_mut());
            borrow.update();
            black_box(borrow);
        })
    });

    group.bench_function("SlotCell::take_put", |b| {
        let cell = SlotCell::new(GameEntity::new());
        b.iter(|| {
            let cell = black_box(&cell);
            let mut val = black_box(cell.take());
            val.update();
            cell.put(black_box(val));
        })
    });

    group.finish();
}

fn bench_large_struct_on_heap(c: &mut Criterion) {
    let mut group = c.benchmark_group("GameEntity_Update_On_Heap");

    group.bench_function("RefCell::borrow_mut", |b| {
        let cell = RefCell::new(Box::new(GameEntity::new()));
        b.iter(|| {
            let cell = black_box(&cell);
            let mut borrow = cell.borrow_mut();
            borrow.update();
            black_box(borrow);
        })
    });

    group.bench_function("SlotCell::take_put", |b| {
        let cell = SlotCell::new(Box::new(GameEntity::new()));
        b.iter(|| {
            let cell = black_box(&cell);
            let mut val = cell.take();
            val.update();
            cell.put(black_box(val));
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_overhead,
    bench_i32_update,
    bench_string_update,
    bench_large_struct,
    bench_large_struct_on_heap
);
criterion_main!(benches);
