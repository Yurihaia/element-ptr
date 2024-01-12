#![no_std]
extern crate core;

/// Returns the address of an inner element without creating unneeded
/// intermediate references.
///
/// The general syntax is
#[cfg_attr(doctest, doc = "````notest")] // don't doctest this.
/// ```
/// element_ptr!(base_ptr => /* element accesses */ )
/// ````
/// where `base_ptr` may be any expression that evaluates to a value of the following types:
/// * [`*const T`]
/// * [`*mut T`]
/// * [`NonNull<T>`]
/// 
/// All accesses (besides a dereference) will maintain that pointer type of the input pointer.
/// This is especially nice with [`NonNull<T>`] because it makes everything involving it much
/// more ergonomic.
/// 
/// ### Element accesses
/// 
/// The following a table describes each of the possible accesses that can be inside the macro.
/// These can all be chained by simply putting one after another.
/// 
/// | Access Kind     | Syntax        |           | Equivalent Pointer Expression                  |
/// |-----------------|---------------|-----------|------------------------------------------------|
/// | Field           | `.field`      |           | <code>[addr_of!]\((*ptr).field)</code>         |
/// | Index           | `[index]`     |           | <code>ptr.[cast::\<T>]\().[add]\(index)</code> |
/// | Add Offset      | `+ count`     | [1](#sl1) | <code>ptr.[add]\(count)</code>                 |
/// | Sub Offset      | `- count`     | [1](#sl1) | <code>ptr.[sub]\(count)</code>                 |
/// | Byte Add Offset | `u8+ bytes`   | [1](#sl1) | <code>ptr.[byte_add]\(bytes)</code>            |
/// | Byte Sub Offset | `u8- bytes`   | [1](#sl1) | <code>ptr.[byte_sub]\(bytes)</code>            |
/// | Cast            | `as T =>`     | [2](#sl2) | <code>ptr.[cast::\<T>]\()</code>               |
/// | Dereference     | `.*`          | [3](#sl3) | <code>ptr.[read]\()</code>                     |
/// | Grouping        | `( ... )`     |           | Just groups the inner accesses for clarity.    |
/// 
/// 1. <span id="sl1">
///     `count`/`bytes` may either be an integer literal or an expression wrapped in parentheses.
///     </span>
/// 2. <span id="sl2">
///     The `=>` may be omitted if the cast is the last access in a group.
///     </span>
/// 3. <span id="sl3">
///     A dereference may return a value that is not a pointer only if it is the final access in the macro.
///     In general it is encouraged to not do this and only use deferencing for inner pointers.
///     </span>
///
/// # Safety
/// * All of the [requirements][offsetreq] for [`offset()`] must be upheld. This is relevant for every
///     access except for dereferencing, grouping, and casting.
/// * The derefence access (`.*`) unconditionally reads from the pointer, and must not violate
///     any [requirements][readreq] related to that.
/// 
/// # Examples
/// 
/// The following example should give you a general sense of what the macro is capable of,
/// as well as a pretty good reference for how to use it.
/// 
/// ```
/// use element_ptr::element_ptr;
/// 
/// use std::{
///     alloc::{alloc, dealloc, Layout, handle_alloc_error},
///     ptr,
/// };
/// 
/// struct Example {
///     field_one: u32,
///     uninit: u32,
///     child_struct: ChildStruct,
///     another: *mut Example,
/// }
/// 
/// struct ChildStruct {
///     data: [&'static str; 6],
/// }
/// 
/// let example = unsafe {
///     // allocate two `Example`s on the heap, and then initialize them part by part.
///     let layout = Layout::new::<Example>();
///     
///     let example = alloc(layout).cast::<Example>();
///     if example.is_null() { handle_alloc_error(layout) };
/// 
///     let other_example = alloc(layout).cast::<Example>();
///     if other_example.is_null() { handle_alloc_error(layout) };
///     
///     // Get the pointer to `field_one` and initialize it.
///     element_ptr!(example => .field_one).write(100u32);
///     // But the `uninit` field isn't initialized.
///     // We can't take a reference to the struct without causing UB!
///     
///     // Now initialize the child struct.
///     let string = "It is normally such a pain to manipulate raw pointers, isn't it?";
///     
///     // Get each word from the sentence
///     for (index, word) in string.split(' ').enumerate() {
///         // and push alternating words to each child struct.
///         if index % 2 == 0 {
///             // The index can be any arbitrary expression that evaluates to an usize.
///             element_ptr!(example => .child_struct.data[index / 2]).write(word);
///         } else {
///             element_ptr!(other_example => .child_struct.data[index / 2]).write(word);
///         }
///     }
///     
///     element_ptr!(example => .another).write(other_example);
///     
///     example
/// };
/// 
/// 
/// // Now that the data is initialized, we can read data from the structs.
/// 
/// unsafe {
///     // The `element_ptr!` macro will get a raw pointer to the data.
///     let field_one_ptr: *mut u32 = element_ptr!(example => .field_one);
///     
///     // This means you can even get a pointer to a field that is not initialized.
///     let uninit_field_ptr: *mut u32 = element_ptr!(example => .uninit);
///     
///     assert_eq!(*field_one_ptr, 100);
/// 
///     let seventh_word = element_ptr!(example => .child_struct.data[3]);
///     
///     assert_eq!(*seventh_word, "to");
///     
///     // The `.*` access is used here to go into the pointer to `other_example`.
///     // Note that this requires the field `another` to be initialized, but not any
///     // of the other fields in `example`.
///     // As long as you don't use `.*`, you can be confident that no data will ever
///     // be dereferenced.
///     
///     let second_word = element_ptr!(
///         example => .another.*.child_struct.data[0]
///     );
/// 
///     assert_eq!(*second_word, "is");
/// 
///     // Now lets deallocate everything so MIRI doesn't yell at me for leaking memory.
///     let layout = Layout::new::<Example>();
///     
///     // Here as a convenience, we can cast the pointer to another type using `as T`.
///     dealloc(element_ptr!(example => .another.* as u8), layout);
///     // Of course this is simply the same as using `as *mut T`
///     dealloc(example as *mut u8, layout);
/// }
/// ```
/// 
// the following links need to be explicitly put because rustdoc cannot refer to pointer methods.
/// [addr_of!]: core::ptr::addr_of!
/// [read]: https://doc.rust-lang.org/core/primitive.pointer.html#method.read
/// [add]: https://doc.rust-lang.org/core/primitive.pointer.html#method.add
/// [sub]: https://doc.rust-lang.org/core/primitive.pointer.html#method.sub
/// [byte_add]: https://doc.rust-lang.org/core/primitive.pointer.html#method.byte_add
/// [byte_sub]: https://doc.rust-lang.org/core/primitive.pointer.html#method.byte_sub
/// [`offset()`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.offset
/// [offsetreq]: https://doc.rust-lang.org/core/primitive.pointer.html#safety-2
/// [readreq]: https://doc.rust-lang.org/core/ptr/fn.read.html#safety
/// [cast::\<T>]: https://doc.rust-lang.org/core/primitive.pointer.html#method.cast
/// [`*const T`]: https://doc.rust-lang.org/core/primitive.pointer.html
/// [`*mut T`]: https://doc.rust-lang.org/core/primitive.pointer.html
/// [`NonNull<T>`]: core::ptr::NonNull
// #[cfg(not(doctest))] // just don't doctest any of these. Macros are way too hard to do.
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
