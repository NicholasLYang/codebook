# Codebook

Codebook is a tool for generating code tutorials with guaranteed correct
code snippets.

It allows you to add snippets that are then tested for correctness
in both their syntax and semantics.

## Getting Started

To create a codebook, add a `codebook.toml` file to the root of your
example code. Add the files that you wish to check. Currently codebook
only supports Markdown.

```toml create: examples/getting-started/codebook.toml
files = ["chapter-1.md"]
```

To add a snippet to a Markdown, annotate your code block with the action.
There are three actions: Create, Delete and Edit. Create and Delete 
respectively create and delete files. Edit adds or deletes text in 
an existing file.

````markdown create: examples/getting-started/chapter-1.md
```rust create: src/main.rs
fn main() {
    println!("Hello, world!");
}
```

```diff edit: src/main.rs@1
-    println!("Hello, world!");
+    println!("Goodbye, world!");
```

```rust delete: src/main.rs
```
````

Note that edit requires a line number after the file path to indicate where the edits should begin. Also,
if you want pretty diffs on GitHub, you should change the language in the code block to `diff`.

Then, add a test command to run on each snippet:

````diff edit: examples/getting-started/codebook.toml@1
+ [test]
+ command = "cargo check"
````

And run `codebook check` to verify all the snippets!

## TODO

Codebook is very new. Here are some planned goals:

- Write snippets in other file formats like JSX or HTML.
- ~~Allow users to check out the code at each snippet.~~ Implemented with `codebook generate`
- Compile the files into a nicer tutorial output.
- Add ability to skip a snippet

## Acknowledgements

Codebook is inspired by [Bob Nystrom](https://github.com/munificent)'s wonderful book, 
[Crafting Interpreters](https://craftinginterpreters.com/) and its always correct code snippets.
Bob wrote about the book's build system in [this post](https://journal.stuffwithstuff.com/2020/04/05/crafting-crafting-interpreters/),
which inspired me to try my own take on it. Thanks Bob!

