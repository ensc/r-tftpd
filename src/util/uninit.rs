// Remove me when `maybe_uninit_slice` has been stabilized
pub trait AsInit {
    type Ref;

    /// # Safety
    ///
    /// Caller must ensure that memory has been initialized
    unsafe fn assume_init(self) -> Self::Ref;
}

impl <'a, T:Sized> AsInit for &'a [std::mem::MaybeUninit<T>] {
    type Ref = &'a [T];

    unsafe fn assume_init(self) -> Self::Ref {
        unsafe {
            std::mem::transmute::<Self, Self::Ref>(self)
        }
    }
}

impl <'a, T:Sized> AsInit for &'a mut [std::mem::MaybeUninit<T>] {
    type Ref = &'a mut [T];

    unsafe fn assume_init(self) -> Self::Ref {
        unsafe {
            std::mem::transmute::<Self, Self::Ref>(self)
        }
    }
}

// Remove me when feature `maybe_uninit_write_slice` has been stabilized
pub trait CopyInit<T: Copy> {
    fn write_copy_of_slice_x<'a>(&'a mut self, src: &[T]) -> &'a mut [T];
}

impl <T: Copy> CopyInit<T> for [std::mem::MaybeUninit<T>] {
    fn write_copy_of_slice_x<'a>(&'a mut self, src: &[T]) -> &'a mut [T] {
        let src = unsafe {
            std::mem::transmute::<&[T], &[std::mem::MaybeUninit<T>]>(src)
        };

        self.copy_from_slice(src);

        unsafe {
            self.assume_init()
        }
    }
}
