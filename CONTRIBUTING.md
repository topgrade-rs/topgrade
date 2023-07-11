## Contributing to `topgrade`

Thank you for your interest in contributing to `topgrade`! 
We welcome and encourage contributions of all kinds, such as:

1. Issue reports or feature requests
2. Documentation improvements
3. Code (PR or PR Review)

Please follow the [Karma Runner guidelines](http://karma-runner.github.io/6.2/dev/git-commit-msg.html)
for commit messages.

## Adding a new `step`

In `topgrade`'s term, package manager is called `step`.
To add a new `step` to `topgrade`: 

1. Add a new variant to 
   [`enum Step`](https://github.com/topgrade-rs/topgrade/blob/cb7adc8ced8a77addf2cb051d18bba9f202ab866/src/config.rs#L100)

   ```rust
   pub enum Step {
       // Existed steps
       // ...

       // Your new step here!
       // You may want it to be sorted alphabetically because that looks great:)
       Xxx,
   }
   ```

2. Implement the update function

   You need to find the appropriate location where this update function goes, it should be
   a file under [`src/steps`](https://github.com/topgrade-rs/topgrade/tree/master/src/steps),
   the file names are self-explanatory, for example, `step`s related to `zsh` are
   placed in [`steps/zsh.rs`](https://github.com/topgrade-rs/topgrade/blob/master/src/steps/zsh.rs).

   Then you implement the update function, and put it in the file where it belongs.

   ```rust
   pub fn run_xxx(ctx: &ExecutionContext) -> Result<()> {
       // Check if this step is installed, if not, then this update will be skipped.
       let xxx = require("xxx")?;

       // Print the separator
       print_separator("xxx");

       // Invoke the new step to get things updated!
       ctx.run_type()
          .execute("xxx")
          .arg(/* args required by this step */)
          .status_checked()
   }
   ```

   Such a update function would be conventionally named `run_xxx()`, where `xxx`
   is the name of the new step, and it should take a argument of type 
   `&ExecutionContext`, this is adequate for most cases unless some extra stuff is
   needed (You can find some examples where extra arguments are needed 
   [here](https://github.com/topgrade-rs/topgrade/blob/7e48c5dedcfd5d0124bb9f39079a03e27ed23886/src/main.rs#L201-L219)).

   Update function would usually do 3 things:
   1. Check if the step is installed
   2. Output the Separator
   3. Invoke the step

   Still, this is sufficient for most tools, but you may need some extra stuff
   with complicated `step`.

3. Finally, invoke that update function in `main.rs`

   ```rust
   runner.execute(Step::Xxx, "xxx", || ItsModule::run_xxx(&ctx))?;
   ```

   We use [conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)
   to separate the steps, for example, for steps that are Linux-only, it goes
   like this:

   ```
   #[cfg(target_os = "linux")]
   {
       // Xxx is Linux-only
       runner.execute(Step::Xxx, "xxx", || ItsModule::run_xxx(&ctx))?;
   }
   ```

   Congrats, you just added a new `step`:)

## Modification to the configuration entries

If your PR has the configuration options 
(in [`src/config.rs`](https://github.com/topgrade-rs/topgrade/blob/master/src/config.rs)) 
modified:

1. Adding new options
2. Changing the existing options

Be sure to apply your changes to
[`config.example.toml`](https://github.com/topgrade-rs/topgrade/blob/master/config.example.toml),
and have some basic documentations guiding user how to use these options.

## Before you submit your PR

Make sure your patch passes the following tests on your host:

```shell
$ cargo build
$ cargo fmt
$ cargo clippy
$ cargo test
```

Don't worry about other platforms, we have most of them covered in our CI.

## Some tips

1. Locale

   Some `step` respects locale, which means their output can be in language other
   than English, we should not do check on it.

   For example, one may want to check if a tool works by doing this:

   ```rust
   let output = Command::new("xxx").arg("--help").output().unwrap();
   let stdout = from_utf8(output.stdout).expect("Assume it is UTF-8 encoded");

   if stdout.contains("help") {
       // xxx works
   }
   ```

   If `xxx` respects locale, then the above code should work on English system,
   on a system that does not use English, e.g., it uses Chinese, that `"help"` may be 
   translated to `"帮助"`, and the above code won't work.
