use core::cell::Cell;

pub struct SlotCell<T> {
    cell: Cell<Option<T>>,
}

impl<T> SlotCell<T> {
    pub const fn new(val: T) -> Self {
        Self {
            cell: Cell::new(Some(val)),
        }
    }

    pub const fn late() -> Self {
        Self {
            cell: Cell::new(None),
        }
    }

    pub fn take(&self) -> T {
        self.cell.take().expect("The value has already been taken")
    }

    pub fn put(&self, val: T) {
        // SAFETY: This type is !Sync and no other references to the interior data will exist
        // at this point. So it is fine to temporarily check the interior contents without moving out.
        unsafe {
            if (*self.cell.as_ptr()).is_none() {
                panic!("The value had already been put back or was never taken")
            }
        }
        let _ = self.cell.replace(Some(val));
    }

    pub fn set(&self, val: T) {
        // SAFETY: This type is !Sync and no other references to the interior data will exist
        // at this point. So it is fine to temporarily check the interior contents without moving out.
        unsafe {
            if (*self.cell.as_ptr()).is_none() {
                panic!("Value set while already aquired")
            }
        }
        let _ = self.cell.replace(Some(val));
    }

    pub fn swap(&self, other: &Self) {
        // SAFETY: This type is !Sync and no other references to the interior data will exist
        // at this point. So it is fine to temporarily check the interior contents without moving out.
        unsafe {
            if (*self.cell.as_ptr()).is_none() {
                panic!("self is already aquired")
            }
            if (*other.cell.as_ptr()).is_none() {
                panic!("other is already aquired")
            }
        }
        self.cell.swap(&other.cell);
    }
}