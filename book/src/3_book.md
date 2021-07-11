# Book

Mdbook is needed to get running this book:

```none
$ cargo install mdbook
$ cd book
$ mdbook serve --dest-dir ../docs
```

### Building the book

This updates the book so it is updated on push.

// TODO! add this as a pre-commit hook

```
$ cd book
$ mdbook build --dest-dir ../docs
```
