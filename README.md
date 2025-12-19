# SlotCell

`SlotCell<T>` is a high-performance interior mutability container for Rust that enforces **take-put** semantics. It acts as a "lockless mutex" for single-threaded scenarios, providing a alternative to `RefCell` when you need **owned access** to data rather than references.

---

## Key Features

* **Owned Access:** Unlike `RefCell`, which gives you a `RefMut` guard, `SlotCell` allows you to move the value out of the container entirely.
* **Memory Efficient:** In release builds, `SlotCell<T>` is exactly the same size as `Cell<Option<T>>`. It avoids the overhead of runtime borrow counters used by `RefCell`.
* **No Constraints on `T`:** Unlike `Cell<T>`, your type `T` does not need to implement `Copy`, `Clone`, or `Default`.
* **Debug Tracking:** In debug builds, `SlotCell` tracks the file and line number of the last modification. If you try to take a value that is already gone, it tells you exactly where it was taken.

---

## Comparison with RefCell

| Feature | `SlotCell<T>` | `RefCell<T>` |
| --- | --- | --- |
| **Access Pattern** | Take ownership (`T`) | Borrow reference (`&T` / `&mut T`) |
| **Overhead** | Minimal (Enum discriminant) | Borrow counter |
| **Safety Check** | Runtime check on "is empty" | Runtime check on "is borrowed" |
| **Best For** | Small/Medium stack types | Large stack types, multiple reads |

---

## Usage

### Basic Workflow

```rust
use slot_cell::SlotCell;

// Create a cell
let cell = SlotCell::new(String::from("Hello"));

// Take the value (cell is now empty)
let mut s = cell.take();
s.push_str(" World");

// Put it back
cell.put(s);

assert_eq!(cell.take(), "Hello World");

```

### Late Initialization

Useful for types that cannot be initialized until after the parent struct is created.

```rust
let cell = SlotCell::late();

// ... later ...
cell.put(42);

```

---

## Performance Considerations

`SlotCell` is optimized for stack-allocated values. Because `take()` and `put()` move the data:

* **Fast:** For types  bytes (like `i32`, `String`, `Vec`).
* **Slower:** For very large structs (e.g.,  arrays), where the cost of moving data exceeds the cost of `RefCell`'s borrow counting. For large types, consider wrapping them in a `Box` before putting them in a `SlotCell`.

### Benchmarks at a Glance

Measured on a standard Criterion suite comparing `SlotCell<T>::take/put` vs `RefCell<T>::borrow_mut`.

| Type | Scenario | `RefCell` (Time) | `SlotCell` (Time) | Difference |
| --- | --- | --- | --- | --- |
| **i32** | Primitive Update | 1.49 ns | 640.77 ps | ~57% Faster |
| **-** | Access Overhead | 1.31 ns | 586.05 ps | ~55% Faster |
| **String** | Push Char | 1.34 ns | 5.79 ns | ~4.3x Slower |
| **Large Struct (~1KB)** | Update Fields | 2.24 ns | 132.61 ns | ~59x Slower |
| **Box<Large Struct>** | Heap Update | 2.24 ns | 2.23 ns | ~0.4% Faster |


---

## Correctness and Panics

`SlotCell` is strictly `!Sync`. It ensures correctness by panicking if:

1. You call `take()` on an empty cell.
2. You call `put()` on a full cell.
3. You call `replace()` on an empty cell.

In **debug mode**, these panics include the stack trace/location of the code that previously emptied or filled the slot, making logic errors easy to debug.