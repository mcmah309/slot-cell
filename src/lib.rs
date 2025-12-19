use core::cell::Cell;
use core::{mem, panic::Location};

pub struct SlotCell<T> {
    cell: Cell<Option<T>>,
    #[cfg(debug_assertions)]
    last_modified: Cell<Location<'static>>,
}

impl<T> SlotCell<T> {
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn new(val: T) -> Self {
        Self {
            cell: Cell::new(Some(val)),
            #[cfg(debug_assertions)]
            last_modified: Cell::new(Location::caller().clone()),
        }
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn late() -> Self {
        Self {
            cell: Cell::new(None),
            #[cfg(debug_assertions)]
            last_modified: Cell::new(Location::caller().clone()),
        }
    }

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

    #[inline]
    pub fn is_taken(&self) -> bool {
        // SAFETY: This type is !Sync and no other references to the interior data will exist
        // at this point. So it is fine to temporarily check the interior contents without moving out.
        unsafe {
            (*self.cell.as_ptr()).is_some()
        }
    }

    /// Puts a value back after it was taken
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
                panic!("self has already been put back or was never taken.\n{}",self.last_modified_msg());
            }
        }
        let none = self.cell.replace(Some(val));
        debug_assert!(none.is_none());
        // Since we know the value is `None`, this skips the check if a deconstructor is needed
        mem::forget(none);
        #[cfg(debug_assertions)]
        self.last_modified.set(Location::caller().clone());
    }

    /// Sets the untaken value to a new value
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn set(&self, val: T) {
        // SAFETY: This type is !Sync and no other references to the interior data will exist
        // at this point. So it is fine to temporarily check the interior contents without moving out.
        unsafe {
            if (*self.cell.as_ptr()).is_none() {
                #[cfg(not(debug_assertions))]
                panic!("self is already taken or created with `late`.");
                #[cfg(debug_assertions)]
                panic!("self is already taken or created with `late`.\n{}",self.last_modified_msg());
            }
        }
        let _ = self.cell.replace(Some(val));
        #[cfg(debug_assertions)]
        self.last_modified.set(Location::caller().clone());
    }

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
                panic!("self is already taken or created with `late`.\n{}",self.last_modified_msg());
            }
            if (*other.cell.as_ptr()).is_none() {
                #[cfg(not(debug_assertions))]
                panic!("other is already taken.");
                #[cfg(debug_assertions)]
                panic!("other is already taken or created with `late`.\n{}",other.last_modified_msg());
            }
        }
        self.cell.swap(&other.cell);
        #[cfg(debug_assertions)]
        self.last_modified.set(Location::caller().clone());
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
