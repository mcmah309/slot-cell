use core::cell::Cell;
use core::{mem, panic::Location};
use std::fmt::Debug;

/// A cell type that enforces borrowing semantics (take/put) for interior mutability.
///
/// `SlotCell<T>` wraps a value that can be temporarily "taken out" and later "put back".
/// This is useful for scenarios where you need to move a value out of a structure temporarily,
/// perform operations on it, and then return it. In practice `SlotCell` fills the same role as
/// `RefCell`, while being more efficient and allows access to owned values.
///
/// Unlike `Cell<T>` or `Cell<Option<T>>`:
/// - `T` does not need to implement `Copy`/`Clone`/`Default`, or a separate `T` needed, to take
/// the value out.
/// - Implements `Debug`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, and `Default` if `T` does.
/// - Does not implement `Clone` or `Copy`, or require `T` to be `Clone` or `Copy` for certain operations.
/// - Enforces a correct usage patterns that mimics "borrow semantic" with a runtime check.
/// 
/// Unlike `RefCell<T>`:
/// - No borrow counting is used
/// - Owned values rather references are returned
/// 
/// High level
/// - Owned values are used over guards with references/lifetimes. Thus,
/// it is up to the programmer to follow semantics around taking and returning values 
/// or it will panic otherwise.
/// - A value can only be taken once (until put back)
/// - A value *should* be put back before dropping and will panic otherwise
/// (Calling `into_inner` or `mem::forget` is safe way to remove this behavior).
/// - A value can only be put back when the slot is empty
/// - A value can only be replaced when not already taken
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
    cell: Cell<Option<T>>,
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
    /// assert!(!cell.is_taken());
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn new(val: T) -> Self {
        Self {
            cell: Cell::new(Some(val)),
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
    /// let cell: SlotCell<i32> = SlotCell::late();
    /// assert!(cell.is_taken());
    ///
    /// cell.put(42);
    /// assert!(!cell.is_taken());
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn late() -> Self {
        Self {
            cell: Cell::new(None),
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
    /// the cell was created with `late()` and never filled. In debug builds,
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
        #[cfg(not(debug_assertions))]
        let val = self.cell.replace(None).expect(
            "The value had already been taken and never put back, or was create with `late`.",
        );
        #[cfg(debug_assertions)]
        let val = self.cell.replace(None).unwrap_or_else(|| {
            panic!("The value had already been taken and never put back, or was create with `late`\n{}", self.last_modified_msg())
        });
        #[cfg(debug_assertions)]
        self.last_modified.set(Location::caller().clone());
        val
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
    /// assert!(!cell.is_taken());  // Has value, not taken
    ///
    /// let _value = cell.take();
    /// assert!(cell.is_taken());   // Now empty/taken
    /// ```
    #[inline]
    pub fn is_taken(&self) -> bool {
        // SAFETY: This type is !Sync and no other references to the interior data will exist
        // at this point. So it is fine to temporarily check the interior contents without moving out.
        unsafe { (*self.cell.as_ptr()).is_none() }
    }

    /// Puts a value into the cell, filling an empty slot.
    ///
    /// This is used to return a value that was previously removed via `take()`,
    /// or to initialize a cell created with `late()`.
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
    /// let cell = SlotCell::late();
    /// cell.put(42); // Initialize empty slot
    ///
    /// let _ = cell.take();
    /// cell.put(100); // Put back after taking
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn put(&self, val: T) {
        // SAFETY: This type is !Sync and no other references to the interior data will exist
        // at this point. So it is fine to temporarily check the interior contents without moving out.
        unsafe {
            if (*self.cell.as_ptr()).is_some() {
                #[cfg(not(debug_assertions))]
                panic!("self has already been put back or was never taken.");
                #[cfg(debug_assertions)]
                panic!(
                    "self has already been put back or was never taken.\n{}",
                    self.last_modified_msg()
                );
            }
        }
        let none = self.cell.replace(Some(val));
        debug_assert!(none.is_none());
        // Since we know the value is `None`, this skips the check if a de-constructor is needed
        mem::forget(none);
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
        // SAFETY: This type is !Sync and no other references to the interior data will exist
        // at this point. So it is fine to temporarily check the interior contents without moving out.
        unsafe {
            if (*self.cell.as_ptr()).is_none() {
                #[cfg(not(debug_assertions))]
                panic!("self is already taken or created with `late`.");
                #[cfg(debug_assertions)]
                panic!(
                    "self is already taken or created with `late`.\n{}",
                    self.last_modified_msg()
                );
            }
        }
        let val = self.cell.replace(Some(val)).unwrap();
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
        // SAFETY: This type is !Sync and no other references to the interior data will exist
        // at this point. So it is fine to temporarily check the interior contents without moving out.
        unsafe {
            if (*self.cell.as_ptr()).is_none() {
                #[cfg(not(debug_assertions))]
                panic!("self is already taken or created with `late`.");
                #[cfg(debug_assertions)]
                panic!(
                    "self is already taken or created with `late`.\n{}",
                    self.last_modified_msg()
                );
            }
            if (*other.cell.as_ptr()).is_none() {
                #[cfg(not(debug_assertions))]
                panic!("other is already taken.");
                #[cfg(debug_assertions)]
                panic!(
                    "other is already taken or created with `late`.\n{}",
                    other.last_modified_msg()
                );
            }
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
        let val = self.cell.replace(None);
        let val = if let Some(val) = val {
            val
        } else {
            panic!("self is empty, cannot extract inner value.");
        };
        mem::forget(self);
        return val;
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

impl<T> Debug for SlotCell<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut binding = f.debug_struct("SlotCell");
        let val = self.cell.replace(None);
        let r = binding.field("cell", &val);
        self.cell.replace(val);
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
        let val_a = self.cell.replace(None);
        let val_b = other.cell.replace(None);
        let eq = val_a == val_b;
        self.cell.replace(val_a);
        other.cell.replace(val_b);
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
        let val_a = self.cell.replace(None);
        let val_b = other.cell.replace(None);
        let ord = val_a.cmp(&val_b);
        self.cell.replace(val_a);
        other.cell.replace(val_b);
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
        let val_a = self.cell.replace(None);
        let val_b = other.cell.replace(None);
        let ord = val_a.partial_cmp(&val_b);
        self.cell.replace(val_a);
        other.cell.replace(val_b);
        ord
    }
}

impl<T> std::hash::Hash for SlotCell<T>
where
    T: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let val = self.cell.replace(None);
        val.hash(state);
        self.cell.replace(val);
    }
}

impl<T> Default for SlotCell<T> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn default() -> Self {
        Self {
            cell: Cell::new(None),
            #[cfg(debug_assertions)]
            last_modified: Cell::new(Location::caller().clone()),
        }
    }
}

impl<T> From<Cell<Option<T>>> for SlotCell<T> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    fn from(value: Cell<Option<T>>) -> Self {
        Self {
            cell: value,
            #[cfg(debug_assertions)]
            last_modified: Cell::new(Location::caller().clone()),
        }
    }
}

/// We only do this drop check in debug mode since not checking will **not** cause `UB`.
/// But semantically a user should always return the value when taken before the cell is dropped.
/// If this panic is hit, it will likely inform the user of a logical inconsistency bug.
/// Failing early to more easily detect the bug.
#[cfg(debug_assertions)]
impl<T> Drop for SlotCell<T> {
    fn drop(&mut self) {
        if self.is_taken() {
            panic!("SlotCell was dropped while still taken. Value was never put back.\n{}", self.last_modified_msg());
        }
    }
}