#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

use core::cell::Cell;
use core::fmt::Debug;
use core::mem::MaybeUninit;
use core::panic::Location;

/// A cell type that enforces borrowing semantics (take/put) for interior mutability.
///
/// `SlotCell<T>` wraps a value that can be temporarily "taken out" and later "put back".
/// This is useful for scenarios where you need to move a value out of a structure temporarily,
/// perform operations on it, and then return it. In practice `SlotCell` fills the same role as
/// `RefCell` and acts more like a "lockless mutex", while:
/// - Since backed by a simple [`Cell`]
///     - Potentially more memory efficient depending on alignment
///     - Faster for small stack values (1 register, <= 8 bytes)
///     - Comparable for medium stack values (2 - 3 registers, <= 24 bytes)
///     - Slower for large stack values (Consider `RefCell` or
///     moving data to heap if performance is the main concern)
/// - **allowing owned access.**
///
/// Unlike `Cell<T>` or `Cell<Option<T>>`:
/// - `T` does not need to implement `Copy`/`Clone`/`Default`, or a separate `T` needed, to take
/// the value out.
/// - Implements `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and `Default` if `T` does.
/// - Does not implement `Clone` or `Copy`, or require `T` to be `Clone` or `Copy` for certain operations.
/// - Enforces a correct usage patterns that mimics "borrow semantics" with a runtime check.
///
/// Unlike `RefCell<T>`:
/// - No borrow counting is used
/// - Owned values rather references are returned
/// - No multiple read references
///
/// High level
/// - Owned values are used over guards with references/lifetimes. Thus,
/// it is up to the programmer to follow semantics around taking and returning values
/// or it will panic otherwise.
/// - A value can only be taken once (until put back)
/// - A value can only be put back when the slot is empty
/// - A value can only be replaced when not already empty
///
/// In debug builds, `SlotCell` tracks the location of the last modification, providing
/// helpful panic messages when usage rules are violated.
///
/// # Examples
///
/// ```
/// # use slot_cell::SlotCell;
///
/// let cell = SlotCell::new(42);
/// let value = cell.take();
/// assert_eq!(value, 42);
///
/// // Put the value back
/// cell.put(100);
///
/// // Take it again
/// let value = cell.take();
/// assert_eq!(value, 100);
/// ```
///
/// # Panics
///
/// This type will panic if usage rules are violated (taking when empty, putting when full, etc.).
/// In debug builds, panic messages include the location of the last modification.
pub struct SlotCell<T> {
    is_empty: Cell<bool>,
    cell: Cell<MaybeUninit<T>>,
    #[cfg(debug_assertions)]
    last_modified: Cell<Location<'static>>,
}

impl<T> SlotCell<T> {
    /// Creates a new `SlotCell` containing the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slot_cell::SlotCell;
    ///
    /// let cell = SlotCell::new(42);
    /// assert!(!cell.is_empty());
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn new(val: T) -> Self {
        Self {
            is_empty: Cell::new(false),
            cell: Cell::new(MaybeUninit::new(val)),
            #[cfg(debug_assertions)]
            last_modified: Cell::new(Location::caller().clone()),
        }
    }

    /// Creates a new `SlotCell` that starts empty (without a value).
    ///
    /// This is useful for late initialization patterns where the value
    /// will be provided later via `put()`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slot_cell::SlotCell;
    ///
    /// let cell: SlotCell<i32> = SlotCell::empty();
    /// assert!(cell.is_empty());
    ///
    /// cell.put(42);
    /// assert!(!cell.is_empty());
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn empty() -> Self {
        Self {
            is_empty: Cell::new(true),
            cell: Cell::new(MaybeUninit::uninit()),
            #[cfg(debug_assertions)]
            last_modified: Cell::new(Location::caller().clone()),
        }
    }

    /// Takes the value out of the cell, leaving it empty.
    ///
    /// After calling this method, the cell will be empty until `put()` is called.
    ///
    /// # Panics
    ///
    /// Panics if the value has already been taken (and not put back), or if
    /// the cell was created with `empty()` and never filled. In debug builds,
    /// the panic message includes the location of the last modification.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slot_cell::SlotCell;
    ///
    /// let cell = SlotCell::new(42);
    /// let value = cell.take();
    /// assert_eq!(value, 42);
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn take(&self) -> T {
        if self.is_empty.get() {
            #[cfg(not(debug_assertions))]
            panic!(
                "The value has already been taken and never put back, or was create with `empty`."
            );
            #[cfg(debug_assertions)]
            panic!(
                "The value has already been taken and never put back, or was create with `empty`\n{}",
                self.last_modified_msg()
            )
        }
        let val = self.take_unchecked();
        val
    }

    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    fn take_unchecked(&self) -> T {
        debug_assert!(!self.is_empty.get());
        let val = self.cell.replace(MaybeUninit::uninit());
        self.is_empty.set(true);
        #[cfg(debug_assertions)]
        self.last_modified.set(Location::caller().clone());
        unsafe { val.assume_init() }
    }

    /// Checks whether the cell is currently empty (value has been taken).
    ///
    /// Returns `true` if the cell currently contains a value, `false` if it's empty.
    ///
    /// Note: Despite the method name, this returns `true` when the value is present
    /// and `false` when taken/empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slot_cell::SlotCell;
    ///
    /// let cell = SlotCell::new(42);
    /// assert!(!cell.is_empty());  // Has value, not taken
    ///
    /// let _value = cell.take();
    /// assert!(cell.is_empty());   // Now empty/taken
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.is_empty.get()
    }

    /// Puts a value into the cell, filling an empty slot.
    ///
    /// This is used to return a value that was previously removed via `take()`,
    /// or to initialize a cell created with `empty()`.
    ///
    /// # Panics
    ///
    /// Panics if the cell is already full. `SlotCell` enforces that a value
    /// must be taken before a new one can be put back. In debug builds,
    /// the panic message includes the location of the last modification.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slot_cell::SlotCell;
    ///
    /// let cell = SlotCell::empty();
    /// cell.put(42); // Initialize empty slot
    ///
    /// let _ = cell.take();
    /// cell.put(100); // Put back after taking
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn put(&self, val: T) {
        if !self.is_empty.get() {
            #[cfg(not(debug_assertions))]
            panic!("self has already been put back or was never taken.");
            #[cfg(debug_assertions)]
            panic!(
                "self has already been put back or was never taken.\n{}",
                self.last_modified_msg()
            );
        }
        self.put_unchecked(val);
    }

    #[inline(always)]
    #[cfg_attr(debug_assertions, track_caller)]
    fn put_unchecked(&self, val: T) {
        debug_assert!(self.is_empty.get());
        let _ = self.cell.replace(MaybeUninit::new(val));
        self.is_empty.set(false);
        #[cfg(debug_assertions)]
        self.last_modified.set(Location::caller().clone());
    }

    /// Replaces the current value in the cell with a new one.
    ///
    /// Unlike `put()`, this method requires the cell to currently contain a value.
    /// It is used to update the contents without changing the "filled" state
    /// of the slot.
    ///
    /// # Panics
    ///
    /// Panics if the cell is currently empty (taken). To fill an empty cell,
    /// use `put()` instead. In debug builds, the panic message includes the
    /// location of the last modification.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slot_cell::SlotCell;
    ///
    /// let cell = SlotCell::new(42);
    /// cell.replace(100);
    ///
    /// assert_eq!(cell.take(), 100);
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn replace(&self, val: T) -> T {
        if self.is_empty.get() {
            #[cfg(not(debug_assertions))]
            panic!("self is already taken or created with `empty`.");
            #[cfg(debug_assertions)]
            panic!(
                "self is already taken or created with `empty`.\n{}",
                self.last_modified_msg()
            );
        }
        let val = self.cell.replace(MaybeUninit::new(val));
        let val = unsafe { val.assume_init() };
        #[cfg(debug_assertions)]
        self.last_modified.set(Location::caller().clone());
        val
    }

    /// Swaps the values between two full `SlotCell`s.
    ///
    /// This efficiently exchanges the contents of `self` and `other` without
    /// requiring an intermediate `take()` or `put()` call.
    ///
    /// # Panics
    ///
    /// Panics if either `self` or `other` is currently empty (taken). In debug
    /// builds, the panic message includes the location of the last modification
    /// for the empty cell.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slot_cell::SlotCell;
    ///
    /// let cell_a = SlotCell::new(1);
    /// let cell_b = SlotCell::new(2);
    ///
    /// cell_a.swap(&cell_b);
    ///
    /// assert_eq!(cell_a.take(), 2);
    /// assert_eq!(cell_b.take(), 1);
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn swap(&self, other: &Self) {
        if self.is_empty.get() {
            #[cfg(not(debug_assertions))]
            panic!("self is already taken or created with `empty`.");
            #[cfg(debug_assertions)]
            panic!(
                "self is already taken or created with `empty`.\n{}",
                self.last_modified_msg()
            );
        }
        if other.is_empty.get() {
            #[cfg(not(debug_assertions))]
            panic!("other is already taken or created with `empty`.");
            #[cfg(debug_assertions)]
            panic!(
                "other is already taken or created with `empty`.\n{}",
                other.last_modified_msg()
            );
        }
        self.cell.swap(&other.cell);
        #[cfg(debug_assertions)]
        self.last_modified.set(Location::caller().clone());
    }

    /// Unwraps the value, consuming the cell.
    ///
    /// # Panics
    ///
    /// Panics if `self` is empty (taken). In debug
    /// builds, the panic message includes the location of the last modification
    /// for the empty cell.
    ///
    /// # Examples
    ///
    /// ```
    /// # use slot_cell::SlotCell;
    ///
    /// let c = SlotCell::new(5);
    /// let five = c.into_inner();
    ///
    /// assert_eq!(five, 5);
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn into_inner(self) -> T {
        self.take()
    }

    #[cfg(debug_assertions)]
    fn last_modified_msg(&self) -> String {
        let last_modified = self.last_modified.get();
        format!(
            "Last modified at {}:{}:{}",
            last_modified.file(),
            last_modified.line(),
            last_modified.column(),
        )
    }
}

impl<T> Drop for SlotCell<T> {
    fn drop(&mut self) {
        if self.is_empty.get() {
            return;
        }
        // Allow drop code to run
        let _ = self.take_unchecked();
    }
}

impl<T> Debug for SlotCell<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut binding = f.debug_struct("SlotCell");
        let r = if self.is_empty.get() {
            binding.field("cell", &"TAKEN")
        } else {
            let val = self.take_unchecked();
            let r = binding.field("cell", &val);
            self.put_unchecked(val);
            r
        };
        #[cfg(debug_assertions)]
        let r = r.field("last_modified", &self.last_modified);
        r.finish()
    }
}

impl<T> Eq for SlotCell<T> where T: Eq {}

impl<T> PartialEq for SlotCell<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        let self_is_empty = self.is_empty.get();
        let other_is_empty = other.is_empty.get();
        if self_is_empty && other_is_empty {
            return true;
        }
        if self_is_empty || other_is_empty {
            return false;
        }
        let val_a = self.take_unchecked();
        let val_b = other.take_unchecked();
        let eq = val_a == val_b;
        self.put_unchecked(val_a);
        other.put_unchecked(val_b);
        eq
    }
}

impl<T> Ord for SlotCell<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if std::ptr::eq(self, other) {
            return std::cmp::Ordering::Equal;
        }
        let self_is_empty = self.is_empty.get();
        let other_is_empty = other.is_empty.get();
        if self_is_empty && other_is_empty {
            return std::cmp::Ordering::Equal;
        }
        if self_is_empty {
            return std::cmp::Ordering::Less;
        }
        if other_is_empty {
            return std::cmp::Ordering::Greater;
        }
        let val_a = self.take_unchecked();
        let val_b = other.take_unchecked();
        let ord = val_a.cmp(&val_b);
        self.put_unchecked(val_a);
        other.put_unchecked(val_b);
        ord
    }
}

impl<T> PartialOrd for SlotCell<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if std::ptr::eq(self, other) {
            return Some(std::cmp::Ordering::Equal);
        }
        let self_is_empty = self.is_empty.get();
        let other_is_empty = other.is_empty.get();
        if self_is_empty && other_is_empty {
            return Some(std::cmp::Ordering::Equal);
        }
        if self_is_empty {
            return Some(std::cmp::Ordering::Less);
        }
        if other_is_empty {
            return Some(std::cmp::Ordering::Greater);
        }
        let val_a = self.take_unchecked();
        let val_b = other.take_unchecked();
        let ord = val_a.partial_cmp(&val_b);
        self.put_unchecked(val_a);
        other.put_unchecked(val_b);
        ord
    }
}

impl<T> std::hash::Hash for SlotCell<T>
where
    T: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.is_empty.get() {
            0usize.hash(state);
        } else {
            let val = self.take_unchecked();
            val.hash(state);
            self.put_unchecked(val);
        }
    }
}

impl<T> Default for SlotCell<T> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn default() -> Self {
        Self {
            is_empty: Cell::new(true),
            cell: Cell::new(MaybeUninit::uninit()),
            #[cfg(debug_assertions)]
            last_modified: Cell::new(Location::caller().clone()),
        }
    }
}

impl<T> From<T> for SlotCell<T> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> From<Option<T>> for SlotCell<T> {
    #[cfg_attr(debug_assertions, track_caller)]
    fn from(value: Option<T>) -> Self {
        let (is_empty, cell) = match value {
            Some(value) => (false, MaybeUninit::new(value)),
            None => (true, MaybeUninit::uninit()),
        };
        Self {
            is_empty: Cell::new(is_empty),
            cell: Cell::new(cell),
            #[cfg(debug_assertions)]
            last_modified: Cell::new(Location::caller().clone()),
        }
    }
}

impl<T> From<SlotCell<T>> for Option<T> {
    fn from(value: SlotCell<T>) -> Self {
        if value.is_empty() {
            None
        } else {
            Some(value.into_inner())
        }
    }
}

// /// We only do this drop check in debug mode since not checking will **not** cause `UB`.
// /// But semantically a user should always return the value when taken before the cell is dropped.
// /// If this panic is hit, it will likely inform the user of a logical inconsistency bug.
// /// Failing early to more easily detect the bug.
// #[cfg(debug_assertions)]
// impl<T> Drop for SlotCell<T> {
//     fn drop(&mut self) {
//         if self.is_empty() {
//             panic!("SlotCell was dropped while still taken. Value was never put back.\n{}", self.last_modified_msg());
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_take() {
        let cell = SlotCell::new(10);
        assert!(!cell.is_empty());
        assert_eq!(cell.take(), 10);
        assert!(cell.is_empty());
    }

    #[test]
    fn test_empty_and_put() {
        let cell: SlotCell<i32> = SlotCell::empty();
        assert!(cell.is_empty());
        cell.put(20);
        assert!(!cell.is_empty());
        assert_eq!(cell.take(), 20);
    }

    #[test]
    fn test_replace() {
        let cell = SlotCell::new(1);
        let old = cell.replace(2);
        assert_eq!(old, 1);
        assert_eq!(cell.take(), 2);
    }

    #[test]
    fn test_swap() {
        let a = SlotCell::new(1);
        let b = SlotCell::new(2);
        a.swap(&b);
        assert_eq!(a.take(), 2);
        assert_eq!(b.take(), 1);
    }

    #[test]
    fn test_into_inner() {
        let cell = SlotCell::new(String::from("hello"));
        let s = cell.into_inner();
        assert_eq!(s, "hello");
    }

    // --- Panic Tests ---

    #[test]
    #[should_panic]
    fn test_panic_take_empty() {
        let cell = SlotCell::new(5);
        let _ = cell.take();
        let _ = cell.take(); // Should panic
    }

    #[test]
    #[should_panic]
    fn test_panic_put_full() {
        let cell = SlotCell::new(5);
        cell.put(10);
    }

    #[test]
    #[should_panic]
    fn test_panic_replace_empty() {
        let cell: SlotCell<i32> = SlotCell::empty();
        cell.replace(10);
    }

    // --- Trait Tests ---

    #[test]
    fn test_equality() {
        let a = SlotCell::new(5);
        let b = SlotCell::new(5);
        let c = SlotCell::new(6);
        assert_eq!(a, b);
        assert_ne!(a, c);

        // Test equality with empty slots (they are equal)
        let _ = a.take();
        let _ = b.take();
        assert_eq!(a, b);
    }

    #[test]
    fn test_ordering() {
        let a = SlotCell::new(10);
        let b = SlotCell::new(20);
        assert!(a < b);
    }

    #[test]
    fn test_debug_format() {
        let cell = SlotCell::new(42);
        let debug_str = format!("{:?}", cell);
        assert!(debug_str.contains("SlotCell"));
        assert!(debug_str.contains("42"));
    }

    #[test]
    fn test_slotcell_vs_refcell_size() {
        use core::cell::{Cell, RefCell};
        use core::mem::size_of;

        type T = u32;

        let slotcell_size = size_of::<SlotCell<T>>();
        let refcell_size = size_of::<RefCell<T>>();
        let cell_option_size = size_of::<Cell<Option<T>>>();

        #[cfg(debug_assertions)]
        {
            let location_cell_size = size_of::<Cell<Location<'static>>>();

            assert_eq!(
                slotcell_size,
                cell_option_size + location_cell_size,
                "SlotCell<T> should be Cell<Option<T>> + debug tracking"
            );
        }

        #[cfg(not(debug_assertions))]
        {
            assert_eq!(
                slotcell_size, cell_option_size,
                "SlotCell<T> should be exactly Cell<Option<T>> in release mode"
            );

            assert!(
                refcell_size > slotcell_size,
                "RefCell<T> should be larger than SlotCell<T> in release mode. Got {} and {}",
                refcell_size,
                slotcell_size
            );
        }
    }
}
