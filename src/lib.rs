#![no_std]
extern crate core;

/// Returns the address of an inner element without created unneeded
/// intermediate references.
///
/// The general syntax is
/// ```
/// element_ptr!(base_ptr => /* element accesses */ )
/// ````
/// The possible element accesses are:
/// * `. $field`: Gets a pointer to the field specified by the field name
///     of the struct behind the pointer.
/// * `. $index`: Same as `. $field` but with a tuple index instead of a named struct field.
/// * `[ $index ]`: Gets an element from a pointer to an array or slice at the specified index.
/// * `+ $offset`: Equivalent to [`pointer::add()`]. See its documentation for more info.
/// * `- $offset`: Equivalent to [`pointer::sub()`]. See its documentation for more info.
/// * `u8+ $offset`: Equivalent to [`pointer::byte_add()`]. See its documentation for more info.
/// * `u8- $offset`: Equivalent to [`pointer::byte_sub()`]. See its documentation for more info.
/// * `as $type =>`: Casts the pointer to a pointer with a pointee type of `$type`.
///     If this is the last access within a group, the `=>` may be omitted.
/// * `( $accesses )`: Groups accesses. Has no effect on the order in which accesses are applied,
///     it just exists to allow for syntactic clarity.
/// * `.*`: [Reads] the value behind the pointer. This should generally only be used
///     for moving into a child pointer.
///
/// If some access returns a value that is not a pointer (meaning `.*` or a group containing it
/// as the last access), it will be a compiler error to have any accesses afterwards.
///
/// # Safety
/// * Every intermediate pointer and the final pointer must remain within the bounds of the same
///     allocated object. See [`pointer::offset()`] for more information.
/// * The `.*` element access unconditionally reads the value from memory.
///     See [`read()`] for more information.
/// * Aside from `.*`, all other element accesses do not read from the memory they are pointing to.
///     They also do not create intermediate references.
///
/// [Reads]: core::ptr::read
/// [`read()`]: core::ptr::read
/// [`pointer::add()`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.add
/// [`pointer::sub()`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.sub
/// [`pointer::byte_add()`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.byte_add
/// [`pointer::byte_sub()`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.byte_sub
/// [`pointer::offset()`]: https://doc.rust-lang.org/nightly/core/primitive.pointer.html#method.offset
#[cfg(not(doctest))] // just don't doctest any of these. Macros are way too hard to do.
pub use element_ptr_macro::element_ptr;

#[doc(hidden)]
pub mod helper {
    use core::{marker::PhantomData, mem::ManuallyDrop};

    pub unsafe trait Mutability {
        type Var<T: ?Sized>;
        type Raw<T: ?Sized>: IsPtr<M = Self, T = T>;
    }
    pub unsafe trait IsPtr: Copy {
        type M: Mutability;
        type T: ?Sized;
    }

    pub enum Const {}
    pub enum Mut {}
    // NonNull is safe here because all of the methods on `Pointer`
    // and related freestanding functions all require the pointer
    // to stay within the bounds of the allocated object.
    // Because the null address is not ever part of an allocated object,
    // this means that as long as a pointer is created from an existing `NonNull`,
    // all uses that would invalidate the `NonNull` would be UB regardless.
    pub enum NonNull {}

    unsafe impl Mutability for Const {
        type Var<T: ?Sized> = fn() -> T;
        type Raw<T: ?Sized> = *const T;
    }
    unsafe impl Mutability for Mut {
        type Var<T: ?Sized> = fn(T) -> T;
        type Raw<T: ?Sized> = *mut T;
    }
    unsafe impl Mutability for NonNull {
        type Var<T: ?Sized> = fn() -> T;
        type Raw<T: ?Sized> = core::ptr::NonNull<T>;
    }

    unsafe impl<T: ?Sized> IsPtr for *mut T {
        type M = Mut;
        type T = T;
    }
    unsafe impl<T: ?Sized> IsPtr for *const T {
        type M = Const;
        type T = T;
    }
    unsafe impl<T: ?Sized> IsPtr for core::ptr::NonNull<T> {
        type M = NonNull;
        type T = T;
    }

    // Store a const pointer to do the manipulations with.
    #[repr(transparent)]
    pub struct Pointer<M: Mutability, T: ?Sized>(*const T, PhantomData<(M, M::Var<T>)>);

    impl<M: Mutability, T> Clone for Pointer<M, T> {
        fn clone(&self) -> Self {
            *self
        }
    }
    impl<M: Mutability, T> Copy for Pointer<M, T> {}

    #[inline(always)]
    pub const fn new_pointer<P: IsPtr>(ptr: P) -> Pointer<P::M, P::T> {
        // Safety
        // `IsPtr` guarantees that `P` may be transmuted into `*const P::T`.
        unsafe { Pointer(transmute_unchecked::<P, *const P::T>(ptr), PhantomData) }
    }

    impl<M: Mutability, T> Pointer<M, T> {
        /// Copies the address and type of a pointer to this pointer, keeping
        /// mutability intact.
        ///
        /// # Safety
        /// * `ptr` must be within the same allocated object as `self`.
        #[inline(always)]
        pub const unsafe fn copy_addr<E: ?Sized>(self, ptr: *const E) -> Pointer<M, E> {
            Pointer(ptr, PhantomData)
        }
        /// Returns the inner pointer type.
        #[inline(always)]
        pub const fn into_inner(self) -> M::Raw<T> {
            // Safety
            // `Pointer<M, T>` can only be created with from a `P: IsPtr`,
            // an `IsPtr` guarantees that `*const T` may be cast to `M::Raw<T>`.
            unsafe { transmute_unchecked(self.0) }
        }
        /// Returns a `*const T` that points to the same place as this pointer.
        #[inline(always)]
        pub const fn into_const(self) -> *const T {
            self.0
        }
        /// Casts this pointer to another type.
        #[inline(always)]
        pub const fn cast<U>(self) -> Pointer<M, U> {
            Pointer(self.0.cast(), PhantomData)
        }
        /// Calculates the offset of this pointer in units of `T`.
        ///
        /// This function is a wrapper around [`pointer::add()`].
        /// See its documentation for more info including the safety requirements.
        ///
        /// [`pointer::add()`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.add
        #[inline(always)]
        pub const unsafe fn add(mut self, count: usize) -> Self {
            self.0 = self.0.add(count);
            self
        }
        /// Calculates the offset of this pointer in units of `T`.
        ///
        /// This function is a wrapper around [`pointer::sub()`].
        /// See its documentation for more info including the safety requirements.
        ///
        /// [`pointer::sub()`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.sub
        #[inline(always)]
        pub const unsafe fn sub(mut self, count: usize) -> Self {
            self.0 = self.0.sub(count);
            self
        }
        /// Calculates the offset of this pointer in units of `T`.
        ///
        /// This function is a wrapper around [`pointer::offset()`].
        /// See its documentation for more info including the safety requirements.
        ///
        /// [`pointer::offset()`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.offset
        #[inline(always)]
        pub const unsafe fn offset(mut self, count: isize) -> Self {
            self.0 = self.0.offset(count);
            self
        }
        /// Calculates the offset of this pointer in bytes.
        ///
        /// This function is a wrapper around [`pointer::byte_add()`].
        /// See its documentation for more info including the safety requirements.
        ///
        /// [`pointer::byte_add()`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.byte_add
        #[inline(always)]
        pub const unsafe fn byte_add(mut self, count: usize) -> Self {
            self.0 = self.0.byte_add(count);
            self
        }
        /// Calculates the offset of this pointer in bytes.
        ///
        /// This function is a wrapper around [`pointer::byte_sub()`].
        /// See its documentation for more info including the safety requirements.
        ///
        /// [`pointer::byte_sub()`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.byte_sub
        #[inline(always)]
        pub const unsafe fn byte_sub(mut self, count: usize) -> Self {
            self.0 = self.0.byte_sub(count);
            self
        }
        /// Calculates the offset of this pointer in bytes.
        ///
        /// This function is a wrapper around [`pointer::byte_offset()`].
        /// See its documentation for more info including the safety requirements.
        ///
        /// [`pointer::byte_offset()`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.byte_offset
        #[inline(always)]
        pub const unsafe fn byte_offset(mut self, count: isize) -> Self {
            self.0 = self.0.byte_offset(count);
            self
        }
        /// Reads the value from behind this pointer.
        ///
        /// This function is a wrapper around [`pointer::read()`].
        /// See its documentation for more info including the safety requirements.
        ///
        /// [`pointer::read()`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.read
        #[inline(always)]
        pub const unsafe fn read(self) -> T {
            self.0.read()
        }
    }

    // This is a freestanding function to make the error message
    // when T doesn't implement `CanIndex` slightly better.
    #[inline(always)]
    pub const unsafe fn index<M: Mutability, T>(
        ptr: Pointer<M, T>,
        index: usize,
    ) -> Pointer<M, T::E>
    where
        T: CanIndex,
    {
        let base = ptr.into_const().cast::<T::E>();
        let ptr = base.add(index);
        Pointer(ptr, PhantomData)
    }

    /// Transmutes from `F` to `T`. All of the normal safety requirements
    /// for transmutations hold here.
    ///
    /// This is just `transmute_copy` except by value.
    pub const unsafe fn transmute_unchecked<F, T>(from: F) -> T {
        #[repr(C)]
        union Transmute<F, T> {
            from: ManuallyDrop<F>,
            to: ManuallyDrop<T>,
        }
        ManuallyDrop::into_inner(
            Transmute {
                from: ManuallyDrop::new(from),
            }
            .to,
        )
    }

    /// A trait to mark which types may be trivially indexed with pointer arithmetic.
    ///
    /// # Safety
    /// * `E` must be the element of the sequence.
    /// * `Self` must be able to be transmuted to a pointer type.
    ///     Specifically, a pointer must reside at offset 0 of `Self`,
    ///     and it must point to a consecutive sequence of `E`s.
    pub unsafe trait CanIndex {
        type E;
    }

    unsafe impl<T, const L: usize> CanIndex for [T; L] {
        type E = T;
    }

    unsafe impl<T> CanIndex for [T] {
        type E = T;
    }

    /// Used to make element_ptr! unsafe and not give a million
    /// different "needs an unsafe block" notification.
    #[doc(hidden)]
    #[inline(always)]
    pub unsafe fn element_ptr_unsafe() {}
}
