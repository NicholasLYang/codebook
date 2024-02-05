# Basic Example

Let's create a new file `src/hello.rs` and add a simple `println!` to it:

```rust create: src/hello.rs
pub fn hello() {
    println!("Hello hello!");
}
```

Now let's add it as a module to the main file and call the function:

```diff insert: src/main.rs@0
+ mod hello;
fn main() {
-     println!("Hello, world!");
+     hello::hello();    
}
```

And finally, let's undo everything

```diff insert: src/main.rs@0
- mod hello;
fn main() {
-     hello::hello();    
+     println!("Hello, world!");
}
```

```rust delete: src/hello.rs
```
