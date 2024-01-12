# Element Pointer

This crate exposes a macro that makes dealing with raw pointers much easier.

One of the common pain points with raw pointers in Rust is the difficulty in handling them safely.
Some examples of the difficulties include:

* It is dangerous and probably wrong to create intermediate references when navigating
    through a structure via a raw pointer. This is why [`addr_of!()`] exists.
    
* Even with [`addr_of!()`], the syntax for repeated uses of it can look quite horrible. Every
    single `.field` access needs to look like `(*ptr).field`, and the macro invocation itself causes
    a lot of clutter.

* [`NonNull<T>`] is incredibly annoying to use. It lacks methods like [`offset()`], and it can't be used
    with the afformentioned [`addr_of!()`]. This means that anything using [`NonNull<T>`] is forced to
    continuously cast to a normal raw pointer and back.

* There are no concise methods to take a `*const [T; L]` and get a raw pointer to one of its elements.
    The correct way to do this is `ptr.cast::<T>().add(index)`, but specifying the cast type is
    verbose, and not doing it may cause confusing compiler errors.

This crate tries to fix these challenges with a comprehensive macro.

## Example

The following example details the basic useage of the macro.

```rust
use element_ptr::element_ptr;

struct BaseStruct {
    first: u32,
    second: ChildStruct,
}

struct ChildStruct {
    elements: [u32; 10],
}

unsafe fn get_child_element_ptr(
    ptr: *const BaseStruct,
    index: usize
) -> *const u32 {
    element_ptr!(ptr => .second.elements[index])
}
```

The macro itself uses a special syntax to describe how the pointer should be moved around.

First, the macro is invoked and is supplied with the base pointer. This may be any expression
that evaluates to a valid pointer type.

```rust
element_ptr!(ptr => /* ... */ )
```

Next, the syntax

```rust
.second
```

makes the pointer move to the address of the `second` field. This is the most basic
kind of manipulation, and probably the one that will be used most often. Instead of an
identifier, an integer may also be used to index into a tuple.

Another field is then accessed. Note how after the previous access,
the pointer was `*const ChildStruct`. This means that `.elements` will access
the `elements` field inside the inner `ChildStruct`. Because the struct is stored by value,
this does not require derefencing the pointer.

Then, an index of the array is accessed with

```rs
[index]
```

The macro will statically check to make sure that index syntax is only used on indexable pointee types.
`index` can be any expression, and not just a variable name or static value.

Finally, the macro returns the pointer to the last accessed subelement, in this case, one of the ten `u32`s
inside `second.elements`.

## Safety

Invoking this macro will always be `unsafe` for a few reasons:

1. [`addr_of!()`], and hence `.field` accesses, require that the resulting pointer
    stays within the bounds of the same [allocated object].
    This is the same requirement that [`offset()`] has.
    
2. Similar to #1, `[index]` accesses also require the resulting pointer to be within bounds.
    Because `index` may be an arbitrary expression, and slices as well as arrays can be indexed,
    this cannot be asserted at compile time.
    
3. This macro supports manipulating [`NonNull<T>`]s, and therefore any offsets could potentially
    cause the pointer to move to null, causing UB. This is almost always the same as #1, because
    the address `0` can never be within bounds.
    
## Syntax & Semantics

There are numerous kinds of element accesses that each can do different things. None of them will ever
derefence the pointer except for `.*`.

| Access Kind     | Syntax        |           | Equivalent Pointer Expression                  |
|-----------------|---------------|-----------|------------------------------------------------|
| Field           | `.field`      |           | <code>[addr_of!]\((*ptr).field)</code>         |
| Index           | `[index]`     |           | <code>ptr.[cast::\<T>]\().[add]\(index)</code> |
| Add Offset      | `+ count`     | [1](#sl1) | <code>ptr.[add]\(count)</code>                 |
| Sub Offset      | `- count`     | [1](#sl1) | <code>ptr.[sub]\(count)</code>                 |
| Byte Add Offset | `u8+ bytes`   | [1](#sl1) | <code>ptr.[byte_add]\(bytes)</code>            |
| Byte Sub Offset | `u8- bytes`   | [1](#sl1) | <code>ptr.[byte_sub]\(bytes)</code>            |
| Cast            | `as T =>`     | [2](#sl2) | <code>ptr.[cast::\<T>]\()</code>               |
| Dereference     | `.*`          | [3](#sl3) | <code>ptr.[read]\()</code>                     |
| Grouping        | `( ... )`     |           | Just groups the inner accesses for clarity.    |


1. <span id="sl1"></span>
    `count`/`bytes` may either be an integer literal or an expression wrapped in parentheses.
2. <span id="sl2"></span>
    The `=>` may be omitted if the cast is the last access in a group.
3. <span id="sl3"></span>
    A dereference may return a value that is not a pointer only if it is the final access in the macro.<br>
    Note that because this calls [`read()`] on the pointer, it can easily lead to duplicate values.
    In general, only use this access on inner pointer types.

[`addr_of!()`]: https://doc.rust-lang.org/core/ptr/macro.addr_of.html
[addr_of!]: https://doc.rust-lang.org/core/ptr/macro.addr_of.html
[`NonNull<T>`]: https://doc.rust-lang.org/core/ptr/struct.NonNull.html
[`offset()`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.offset
[allocated object]: https://doc.rust-lang.org/core/ptr/index.html#allocated-object
[cast::\<T>]: https://doc.rust-lang.org/core/primitive.pointer.html#method.cast
[add]: https://doc.rust-lang.org/core/primitive.pointer.html#method.add
[sub]: https://doc.rust-lang.org/core/primitive.pointer.html#method.add
[byte_add]: https://doc.rust-lang.org/core/primitive.pointer.html#method.byte_add
[byte_sub]: https://doc.rust-lang.org/core/primitive.pointer.html#method.byte_sub
[read]: https://doc.rust-lang.org/core/primitive.pointer.html#method.read
[`read()`]: https://doc.rust-lang.org/core/primitive.pointer.html#method.read