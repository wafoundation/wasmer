# Development Notes

## Ownerships

The `wasm.h` header thankfully defines the `own` “annotation”. It
specifies _who_ owns _what_. For example, in the following code:

```c
WASM_API_EXTERN own wasm_importtype_t* wasm_importtype_new(
  own wasm_name_t* module, own wasm_name_t* name, own wasm_externtype_t*);
```

We must read that `wasm_importtype_new` takes the ownership of all its
three arguments. This function is then responsible to free those
data. We must also read that the returned value is owned by the caller
of this function.

### Rust Pattern

This ownership property translates well in Rust. We have decided to
use the `Box<T>` type to represent an owned pointer. `Box<T>` drops
its content when it's dropped.

Consequently, apart from other patterns, the code above can be written
as follows in Rust:

```rust
#[no_mangle]
pub extern "C" fn wasm_importtype_new(
    module: Box<wasm_name_t>,
    name: Box<wasm_name_t>,
    extern_type: Box<wasm_externtype_t>,
) -> Box<wasm_importtype_t> {
    …
}
```

By reading the code, it is clear that `wasm_importtype_new` takes the
ownership for `module`, `name`, and `extern_type`, and that the result
is owned by the caller.

## Null Pointer

The `wasm.h` header does not say anything about null pointer. The
behavior we agreed on in that passing a null pointer where it is not
expected (i.e. no where) will make the function to return null too
without any error.

### Rust Pattern

A nice type property in Rust is that it is possible to write
`Option<NonNull<T>>` to nicely handle null pointer of kind `T`. For an
argument, it translates as follows:

* When the given pointer is null, the argument holds `None`,
* When the given pointer is not null, the arguments holds
  `Some(NonNull<T>)`.

Considering [the Ownerships Section][#ownerships], if the pointer is
owned, we can also write `Option<Box<T>>`. This pattern is largely
used in this codebase to represent a “nullable” owned
pointer. Consequently, a code like:

```c
WASM_API_EXTERN own wasm_importtype_t* wasm_importtype_new(
  own wasm_name_t* module, own wasm_name_t* name, own wasm_externtype_t*);
```

translates into Rust as:

```rust
#[no_mangle]
pub extern "C" fn wasm_importtype_new(
    module: Option<Box<wasm_name_t>>,
    name: Option<Box<wasm_name_t>>,
    extern_type: Option<Box<wasm_externtype_t>>,
) -> Option<Box<wasm_importtype_t>> {
    Some(Box::new(wasm_importtype_t {
        name: name?,
        module: module?,
        extern_type: extern_type?,
    }))
}
```

What `name?` (and others) means? It is basically [the `Try` trait
implemented for
`Option`](https://doc.rust-lang.org/std/ops/trait.Try.html#impl-Try):
It returns `None` if the value is `None`, otherwise it unwraps the
`Option`.

Because the function returns `Option<Box<T>>`, `None` represents a
null pointer.

## `const *T`

A constant pointer can be interpreted in C as an immutable
pointer. Without the `own` annotation, it means the ownership is not
transfered anywhere (see [the Ownerships Section][#ownerships]).

### Rust Pattern

`const *T` translates to Rust as `&T`, it's a reference.

Note: It could translate to `Option<NonNull<T>>` and then we could
call `x?.as_ref()` to get a `&T`. It could also translate to
`Option<&T>`. Whether we should use such patterns in all the codebase
is still under discussion.