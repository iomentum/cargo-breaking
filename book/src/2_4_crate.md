# Code fetching

Once the crate's metadata has been decoded, cargo-breaking fetches the code for the previous and the next version in a
temporary directory. This is done by copying the `.git` folder in two different directories, and then checking out 
respectively the revspec specified via `-a` / `--against` and the `HEAD` revspec.

cargo-breaking internally supports a second loading source, which is used for loading raw code files directly. This is 
used by the test harness to compare two single-file libraries.